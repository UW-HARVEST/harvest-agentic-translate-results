// This file was generated to match c_src/for-gen.c.
// The pack/unpack routines will not work on big-endian architectures.
#![allow(unused_assignments, unused_variables, unused_mut)]

#[inline] fn read_u32(input: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([input[offset], input[offset + 1], input[offset + 2], input[offset + 3]])
}

#[inline] fn write_u32(output: &mut [u8], offset: usize, val: u32) {
    let bytes = val.to_le_bytes();
    output[offset] = bytes[0];
    output[offset + 1] = bytes[1];
    output[offset + 2] = bytes[2];
    output[offset + 3] = bytes[3];
}

// Write the lower `len` bytes of `val` into `output` starting at `offset`.
#[inline] fn write_partial(output: &mut [u8], offset: usize, val: u32, len: usize) {
    let bytes = val.to_le_bytes();
    for i in 0..len {
        output[offset + i] = bytes[i];
    }
}

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

pub fn pack1_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 1;
    tmp |= (input[2].wrapping_sub(base)) << 2;
    tmp |= (input[3].wrapping_sub(base)) << 3;
    tmp |= (input[4].wrapping_sub(base)) << 4;
    tmp |= (input[5].wrapping_sub(base)) << 5;
    tmp |= (input[6].wrapping_sub(base)) << 6;
    tmp |= (input[7].wrapping_sub(base)) << 7;
    tmp |= (input[8].wrapping_sub(base)) << 8;
    tmp |= (input[9].wrapping_sub(base)) << 9;
    tmp |= (input[10].wrapping_sub(base)) << 10;
    tmp |= (input[11].wrapping_sub(base)) << 11;
    tmp |= (input[12].wrapping_sub(base)) << 12;
    tmp |= (input[13].wrapping_sub(base)) << 13;
    tmp |= (input[14].wrapping_sub(base)) << 14;
    tmp |= (input[15].wrapping_sub(base)) << 15;
    tmp |= (input[16].wrapping_sub(base)) << 16;
    tmp |= (input[17].wrapping_sub(base)) << 17;
    tmp |= (input[18].wrapping_sub(base)) << 18;
    tmp |= (input[19].wrapping_sub(base)) << 19;
    tmp |= (input[20].wrapping_sub(base)) << 20;
    tmp |= (input[21].wrapping_sub(base)) << 21;
    tmp |= (input[22].wrapping_sub(base)) << 22;
    tmp |= (input[23].wrapping_sub(base)) << 23;
    tmp |= (input[24].wrapping_sub(base)) << 24;
    tmp |= (input[25].wrapping_sub(base)) << 25;
    tmp |= (input[26].wrapping_sub(base)) << 26;
    tmp |= (input[27].wrapping_sub(base)) << 27;
    tmp |= (input[28].wrapping_sub(base)) << 28;
    tmp |= (input[29].wrapping_sub(base)) << 29;
    tmp |= (input[30].wrapping_sub(base)) << 30;
    tmp |= (input[31].wrapping_sub(base)) << 31;
    write_partial(output, out_off, tmp, 4);
    4
}

pub fn unpack1_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 1);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 1) & 1);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 2) & 1);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 3) & 1);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 4) & 1);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 5) & 1);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 6) & 1);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 7) & 1);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 8) & 1);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 9) & 1);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 10) & 1);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 11) & 1);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 12) & 1);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 13) & 1);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 14) & 1);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 15) & 1);
    output[16] = base.wrapping_add((read_u32(input, in_off) >> 16) & 1);
    output[17] = base.wrapping_add((read_u32(input, in_off) >> 17) & 1);
    output[18] = base.wrapping_add((read_u32(input, in_off) >> 18) & 1);
    output[19] = base.wrapping_add((read_u32(input, in_off) >> 19) & 1);
    output[20] = base.wrapping_add((read_u32(input, in_off) >> 20) & 1);
    output[21] = base.wrapping_add((read_u32(input, in_off) >> 21) & 1);
    output[22] = base.wrapping_add((read_u32(input, in_off) >> 22) & 1);
    output[23] = base.wrapping_add((read_u32(input, in_off) >> 23) & 1);
    output[24] = base.wrapping_add((read_u32(input, in_off) >> 24) & 1);
    output[25] = base.wrapping_add((read_u32(input, in_off) >> 25) & 1);
    output[26] = base.wrapping_add((read_u32(input, in_off) >> 26) & 1);
    output[27] = base.wrapping_add((read_u32(input, in_off) >> 27) & 1);
    output[28] = base.wrapping_add((read_u32(input, in_off) >> 28) & 1);
    output[29] = base.wrapping_add((read_u32(input, in_off) >> 29) & 1);
    output[30] = base.wrapping_add((read_u32(input, in_off) >> 30) & 1);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 31) & 1);
    4
}

pub fn pack2_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 2;
    tmp |= (input[2].wrapping_sub(base)) << 4;
    tmp |= (input[3].wrapping_sub(base)) << 6;
    tmp |= (input[4].wrapping_sub(base)) << 8;
    tmp |= (input[5].wrapping_sub(base)) << 10;
    tmp |= (input[6].wrapping_sub(base)) << 12;
    tmp |= (input[7].wrapping_sub(base)) << 14;
    tmp |= (input[8].wrapping_sub(base)) << 16;
    tmp |= (input[9].wrapping_sub(base)) << 18;
    tmp |= (input[10].wrapping_sub(base)) << 20;
    tmp |= (input[11].wrapping_sub(base)) << 22;
    tmp |= (input[12].wrapping_sub(base)) << 24;
    tmp |= (input[13].wrapping_sub(base)) << 26;
    tmp |= (input[14].wrapping_sub(base)) << 28;
    tmp |= (input[15].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[16].wrapping_sub(base)) << 0;
    tmp |= (input[17].wrapping_sub(base)) << 2;
    tmp |= (input[18].wrapping_sub(base)) << 4;
    tmp |= (input[19].wrapping_sub(base)) << 6;
    tmp |= (input[20].wrapping_sub(base)) << 8;
    tmp |= (input[21].wrapping_sub(base)) << 10;
    tmp |= (input[22].wrapping_sub(base)) << 12;
    tmp |= (input[23].wrapping_sub(base)) << 14;
    tmp |= (input[24].wrapping_sub(base)) << 16;
    tmp |= (input[25].wrapping_sub(base)) << 18;
    tmp |= (input[26].wrapping_sub(base)) << 20;
    tmp |= (input[27].wrapping_sub(base)) << 22;
    tmp |= (input[28].wrapping_sub(base)) << 24;
    tmp |= (input[29].wrapping_sub(base)) << 26;
    tmp |= (input[30].wrapping_sub(base)) << 28;
    tmp |= (input[31].wrapping_sub(base)) << 30;
    write_partial(output, out_off, tmp, 4);
    8
}

pub fn unpack2_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 3);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 2) & 3);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 4) & 3);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 6) & 3);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 8) & 3);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 10) & 3);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 12) & 3);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 14) & 3);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 16) & 3);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 18) & 3);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 20) & 3);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 22) & 3);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 24) & 3);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 26) & 3);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 28) & 3);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 30) & 3);
    in_off += 4;
    output[16] = base.wrapping_add((read_u32(input, in_off) >> 0) & 3);
    output[17] = base.wrapping_add((read_u32(input, in_off) >> 2) & 3);
    output[18] = base.wrapping_add((read_u32(input, in_off) >> 4) & 3);
    output[19] = base.wrapping_add((read_u32(input, in_off) >> 6) & 3);
    output[20] = base.wrapping_add((read_u32(input, in_off) >> 8) & 3);
    output[21] = base.wrapping_add((read_u32(input, in_off) >> 10) & 3);
    output[22] = base.wrapping_add((read_u32(input, in_off) >> 12) & 3);
    output[23] = base.wrapping_add((read_u32(input, in_off) >> 14) & 3);
    output[24] = base.wrapping_add((read_u32(input, in_off) >> 16) & 3);
    output[25] = base.wrapping_add((read_u32(input, in_off) >> 18) & 3);
    output[26] = base.wrapping_add((read_u32(input, in_off) >> 20) & 3);
    output[27] = base.wrapping_add((read_u32(input, in_off) >> 22) & 3);
    output[28] = base.wrapping_add((read_u32(input, in_off) >> 24) & 3);
    output[29] = base.wrapping_add((read_u32(input, in_off) >> 26) & 3);
    output[30] = base.wrapping_add((read_u32(input, in_off) >> 28) & 3);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 30) & 3);
    8
}

pub fn pack3_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 3;
    tmp |= (input[2].wrapping_sub(base)) << 6;
    tmp |= (input[3].wrapping_sub(base)) << 9;
    tmp |= (input[4].wrapping_sub(base)) << 12;
    tmp |= (input[5].wrapping_sub(base)) << 15;
    tmp |= (input[6].wrapping_sub(base)) << 18;
    tmp |= (input[7].wrapping_sub(base)) << 21;
    tmp |= (input[8].wrapping_sub(base)) << 24;
    tmp |= (input[9].wrapping_sub(base)) << 27;
    tmp |= (input[10].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (3 - 1);
    tmp |= (input[11].wrapping_sub(base)) << 1;
    tmp |= (input[12].wrapping_sub(base)) << 4;
    tmp |= (input[13].wrapping_sub(base)) << 7;
    tmp |= (input[14].wrapping_sub(base)) << 10;
    tmp |= (input[15].wrapping_sub(base)) << 13;
    tmp |= (input[16].wrapping_sub(base)) << 16;
    tmp |= (input[17].wrapping_sub(base)) << 19;
    tmp |= (input[18].wrapping_sub(base)) << 22;
    tmp |= (input[19].wrapping_sub(base)) << 25;
    tmp |= (input[20].wrapping_sub(base)) << 28;
    tmp |= (input[21].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[21].wrapping_sub(base)) >> (3 - 2);
    tmp |= (input[22].wrapping_sub(base)) << 2;
    tmp |= (input[23].wrapping_sub(base)) << 5;
    tmp |= (input[24].wrapping_sub(base)) << 8;
    tmp |= (input[25].wrapping_sub(base)) << 11;
    tmp |= (input[26].wrapping_sub(base)) << 14;
    tmp |= (input[27].wrapping_sub(base)) << 17;
    tmp |= (input[28].wrapping_sub(base)) << 20;
    tmp |= (input[29].wrapping_sub(base)) << 23;
    tmp |= (input[30].wrapping_sub(base)) << 26;
    tmp |= (input[31].wrapping_sub(base)) << 29;
    write_partial(output, out_off, tmp, 4);
    12
}

pub fn unpack3_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 7);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 3) & 7);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 6) & 7);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 9) & 7);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 12) & 7);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 15) & 7);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 18) & 7);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 21) & 7);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 24) & 7);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 27) & 7);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (3 - 1);
    output[10] = base.wrapping_add(tmp);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 1) & 7);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 4) & 7);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 7) & 7);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 10) & 7);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 13) & 7);
    output[16] = base.wrapping_add((read_u32(input, in_off) >> 16) & 7);
    output[17] = base.wrapping_add((read_u32(input, in_off) >> 19) & 7);
    output[18] = base.wrapping_add((read_u32(input, in_off) >> 22) & 7);
    output[19] = base.wrapping_add((read_u32(input, in_off) >> 25) & 7);
    output[20] = base.wrapping_add((read_u32(input, in_off) >> 28) & 7);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (3 - 2);
    output[21] = base.wrapping_add(tmp);
    output[22] = base.wrapping_add((read_u32(input, in_off) >> 2) & 7);
    output[23] = base.wrapping_add((read_u32(input, in_off) >> 5) & 7);
    output[24] = base.wrapping_add((read_u32(input, in_off) >> 8) & 7);
    output[25] = base.wrapping_add((read_u32(input, in_off) >> 11) & 7);
    output[26] = base.wrapping_add((read_u32(input, in_off) >> 14) & 7);
    output[27] = base.wrapping_add((read_u32(input, in_off) >> 17) & 7);
    output[28] = base.wrapping_add((read_u32(input, in_off) >> 20) & 7);
    output[29] = base.wrapping_add((read_u32(input, in_off) >> 23) & 7);
    output[30] = base.wrapping_add((read_u32(input, in_off) >> 26) & 7);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 29) & 7);
    12
}

pub fn pack4_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 4;
    tmp |= (input[2].wrapping_sub(base)) << 8;
    tmp |= (input[3].wrapping_sub(base)) << 12;
    tmp |= (input[4].wrapping_sub(base)) << 16;
    tmp |= (input[5].wrapping_sub(base)) << 20;
    tmp |= (input[6].wrapping_sub(base)) << 24;
    tmp |= (input[7].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) << 0;
    tmp |= (input[9].wrapping_sub(base)) << 4;
    tmp |= (input[10].wrapping_sub(base)) << 8;
    tmp |= (input[11].wrapping_sub(base)) << 12;
    tmp |= (input[12].wrapping_sub(base)) << 16;
    tmp |= (input[13].wrapping_sub(base)) << 20;
    tmp |= (input[14].wrapping_sub(base)) << 24;
    tmp |= (input[15].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[16].wrapping_sub(base)) << 0;
    tmp |= (input[17].wrapping_sub(base)) << 4;
    tmp |= (input[18].wrapping_sub(base)) << 8;
    tmp |= (input[19].wrapping_sub(base)) << 12;
    tmp |= (input[20].wrapping_sub(base)) << 16;
    tmp |= (input[21].wrapping_sub(base)) << 20;
    tmp |= (input[22].wrapping_sub(base)) << 24;
    tmp |= (input[23].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[24].wrapping_sub(base)) << 0;
    tmp |= (input[25].wrapping_sub(base)) << 4;
    tmp |= (input[26].wrapping_sub(base)) << 8;
    tmp |= (input[27].wrapping_sub(base)) << 12;
    tmp |= (input[28].wrapping_sub(base)) << 16;
    tmp |= (input[29].wrapping_sub(base)) << 20;
    tmp |= (input[30].wrapping_sub(base)) << 24;
    tmp |= (input[31].wrapping_sub(base)) << 28;
    write_partial(output, out_off, tmp, 4);
    16
}

pub fn unpack4_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 15);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 4) & 15);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 8) & 15);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 12) & 15);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 16) & 15);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 20) & 15);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 24) & 15);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 28) & 15);
    in_off += 4;
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 0) & 15);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 4) & 15);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 8) & 15);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 12) & 15);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 16) & 15);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 20) & 15);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 24) & 15);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 28) & 15);
    in_off += 4;
    output[16] = base.wrapping_add((read_u32(input, in_off) >> 0) & 15);
    output[17] = base.wrapping_add((read_u32(input, in_off) >> 4) & 15);
    output[18] = base.wrapping_add((read_u32(input, in_off) >> 8) & 15);
    output[19] = base.wrapping_add((read_u32(input, in_off) >> 12) & 15);
    output[20] = base.wrapping_add((read_u32(input, in_off) >> 16) & 15);
    output[21] = base.wrapping_add((read_u32(input, in_off) >> 20) & 15);
    output[22] = base.wrapping_add((read_u32(input, in_off) >> 24) & 15);
    output[23] = base.wrapping_add((read_u32(input, in_off) >> 28) & 15);
    in_off += 4;
    output[24] = base.wrapping_add((read_u32(input, in_off) >> 0) & 15);
    output[25] = base.wrapping_add((read_u32(input, in_off) >> 4) & 15);
    output[26] = base.wrapping_add((read_u32(input, in_off) >> 8) & 15);
    output[27] = base.wrapping_add((read_u32(input, in_off) >> 12) & 15);
    output[28] = base.wrapping_add((read_u32(input, in_off) >> 16) & 15);
    output[29] = base.wrapping_add((read_u32(input, in_off) >> 20) & 15);
    output[30] = base.wrapping_add((read_u32(input, in_off) >> 24) & 15);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 28) & 15);
    16
}

pub fn pack5_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 5;
    tmp |= (input[2].wrapping_sub(base)) << 10;
    tmp |= (input[3].wrapping_sub(base)) << 15;
    tmp |= (input[4].wrapping_sub(base)) << 20;
    tmp |= (input[5].wrapping_sub(base)) << 25;
    tmp |= (input[6].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (5 - 3);
    tmp |= (input[7].wrapping_sub(base)) << 3;
    tmp |= (input[8].wrapping_sub(base)) << 8;
    tmp |= (input[9].wrapping_sub(base)) << 13;
    tmp |= (input[10].wrapping_sub(base)) << 18;
    tmp |= (input[11].wrapping_sub(base)) << 23;
    tmp |= (input[12].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (5 - 1);
    tmp |= (input[13].wrapping_sub(base)) << 1;
    tmp |= (input[14].wrapping_sub(base)) << 6;
    tmp |= (input[15].wrapping_sub(base)) << 11;
    tmp |= (input[16].wrapping_sub(base)) << 16;
    tmp |= (input[17].wrapping_sub(base)) << 21;
    tmp |= (input[18].wrapping_sub(base)) << 26;
    tmp |= (input[19].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[19].wrapping_sub(base)) >> (5 - 4);
    tmp |= (input[20].wrapping_sub(base)) << 4;
    tmp |= (input[21].wrapping_sub(base)) << 9;
    tmp |= (input[22].wrapping_sub(base)) << 14;
    tmp |= (input[23].wrapping_sub(base)) << 19;
    tmp |= (input[24].wrapping_sub(base)) << 24;
    tmp |= (input[25].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[25].wrapping_sub(base)) >> (5 - 2);
    tmp |= (input[26].wrapping_sub(base)) << 2;
    tmp |= (input[27].wrapping_sub(base)) << 7;
    tmp |= (input[28].wrapping_sub(base)) << 12;
    tmp |= (input[29].wrapping_sub(base)) << 17;
    tmp |= (input[30].wrapping_sub(base)) << 22;
    tmp |= (input[31].wrapping_sub(base)) << 27;
    write_partial(output, out_off, tmp, 4);
    20
}

pub fn unpack5_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 31);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 5) & 31);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 10) & 31);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 15) & 31);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 20) & 31);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 25) & 31);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 3)) << (5 - 3);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 3) & 31);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 8) & 31);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 13) & 31);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 18) & 31);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 23) & 31);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (5 - 1);
    output[12] = base.wrapping_add(tmp);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 1) & 31);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 6) & 31);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 11) & 31);
    output[16] = base.wrapping_add((read_u32(input, in_off) >> 16) & 31);
    output[17] = base.wrapping_add((read_u32(input, in_off) >> 21) & 31);
    output[18] = base.wrapping_add((read_u32(input, in_off) >> 26) & 31);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (5 - 4);
    output[19] = base.wrapping_add(tmp);
    output[20] = base.wrapping_add((read_u32(input, in_off) >> 4) & 31);
    output[21] = base.wrapping_add((read_u32(input, in_off) >> 9) & 31);
    output[22] = base.wrapping_add((read_u32(input, in_off) >> 14) & 31);
    output[23] = base.wrapping_add((read_u32(input, in_off) >> 19) & 31);
    output[24] = base.wrapping_add((read_u32(input, in_off) >> 24) & 31);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (5 - 2);
    output[25] = base.wrapping_add(tmp);
    output[26] = base.wrapping_add((read_u32(input, in_off) >> 2) & 31);
    output[27] = base.wrapping_add((read_u32(input, in_off) >> 7) & 31);
    output[28] = base.wrapping_add((read_u32(input, in_off) >> 12) & 31);
    output[29] = base.wrapping_add((read_u32(input, in_off) >> 17) & 31);
    output[30] = base.wrapping_add((read_u32(input, in_off) >> 22) & 31);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 27) & 31);
    20
}

pub fn pack6_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 6;
    tmp |= (input[2].wrapping_sub(base)) << 12;
    tmp |= (input[3].wrapping_sub(base)) << 18;
    tmp |= (input[4].wrapping_sub(base)) << 24;
    tmp |= (input[5].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (6 - 4);
    tmp |= (input[6].wrapping_sub(base)) << 4;
    tmp |= (input[7].wrapping_sub(base)) << 10;
    tmp |= (input[8].wrapping_sub(base)) << 16;
    tmp |= (input[9].wrapping_sub(base)) << 22;
    tmp |= (input[10].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (6 - 2);
    tmp |= (input[11].wrapping_sub(base)) << 2;
    tmp |= (input[12].wrapping_sub(base)) << 8;
    tmp |= (input[13].wrapping_sub(base)) << 14;
    tmp |= (input[14].wrapping_sub(base)) << 20;
    tmp |= (input[15].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[16].wrapping_sub(base)) << 0;
    tmp |= (input[17].wrapping_sub(base)) << 6;
    tmp |= (input[18].wrapping_sub(base)) << 12;
    tmp |= (input[19].wrapping_sub(base)) << 18;
    tmp |= (input[20].wrapping_sub(base)) << 24;
    tmp |= (input[21].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[21].wrapping_sub(base)) >> (6 - 4);
    tmp |= (input[22].wrapping_sub(base)) << 4;
    tmp |= (input[23].wrapping_sub(base)) << 10;
    tmp |= (input[24].wrapping_sub(base)) << 16;
    tmp |= (input[25].wrapping_sub(base)) << 22;
    tmp |= (input[26].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[26].wrapping_sub(base)) >> (6 - 2);
    tmp |= (input[27].wrapping_sub(base)) << 2;
    tmp |= (input[28].wrapping_sub(base)) << 8;
    tmp |= (input[29].wrapping_sub(base)) << 14;
    tmp |= (input[30].wrapping_sub(base)) << 20;
    tmp |= (input[31].wrapping_sub(base)) << 26;
    write_partial(output, out_off, tmp, 4);
    24
}

pub fn unpack6_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 63);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 6) & 63);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 12) & 63);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 18) & 63);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 24) & 63);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (6 - 4);
    output[5] = base.wrapping_add(tmp);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 4) & 63);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 10) & 63);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 16) & 63);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 22) & 63);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (6 - 2);
    output[10] = base.wrapping_add(tmp);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 2) & 63);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 8) & 63);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 14) & 63);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 20) & 63);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 26) & 63);
    in_off += 4;
    output[16] = base.wrapping_add((read_u32(input, in_off) >> 0) & 63);
    output[17] = base.wrapping_add((read_u32(input, in_off) >> 6) & 63);
    output[18] = base.wrapping_add((read_u32(input, in_off) >> 12) & 63);
    output[19] = base.wrapping_add((read_u32(input, in_off) >> 18) & 63);
    output[20] = base.wrapping_add((read_u32(input, in_off) >> 24) & 63);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (6 - 4);
    output[21] = base.wrapping_add(tmp);
    output[22] = base.wrapping_add((read_u32(input, in_off) >> 4) & 63);
    output[23] = base.wrapping_add((read_u32(input, in_off) >> 10) & 63);
    output[24] = base.wrapping_add((read_u32(input, in_off) >> 16) & 63);
    output[25] = base.wrapping_add((read_u32(input, in_off) >> 22) & 63);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (6 - 2);
    output[26] = base.wrapping_add(tmp);
    output[27] = base.wrapping_add((read_u32(input, in_off) >> 2) & 63);
    output[28] = base.wrapping_add((read_u32(input, in_off) >> 8) & 63);
    output[29] = base.wrapping_add((read_u32(input, in_off) >> 14) & 63);
    output[30] = base.wrapping_add((read_u32(input, in_off) >> 20) & 63);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 26) & 63);
    24
}

pub fn pack7_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 7;
    tmp |= (input[2].wrapping_sub(base)) << 14;
    tmp |= (input[3].wrapping_sub(base)) << 21;
    tmp |= (input[4].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (7 - 3);
    tmp |= (input[5].wrapping_sub(base)) << 3;
    tmp |= (input[6].wrapping_sub(base)) << 10;
    tmp |= (input[7].wrapping_sub(base)) << 17;
    tmp |= (input[8].wrapping_sub(base)) << 24;
    tmp |= (input[9].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (7 - 6);
    tmp |= (input[10].wrapping_sub(base)) << 6;
    tmp |= (input[11].wrapping_sub(base)) << 13;
    tmp |= (input[12].wrapping_sub(base)) << 20;
    tmp |= (input[13].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (7 - 2);
    tmp |= (input[14].wrapping_sub(base)) << 2;
    tmp |= (input[15].wrapping_sub(base)) << 9;
    tmp |= (input[16].wrapping_sub(base)) << 16;
    tmp |= (input[17].wrapping_sub(base)) << 23;
    tmp |= (input[18].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[18].wrapping_sub(base)) >> (7 - 5);
    tmp |= (input[19].wrapping_sub(base)) << 5;
    tmp |= (input[20].wrapping_sub(base)) << 12;
    tmp |= (input[21].wrapping_sub(base)) << 19;
    tmp |= (input[22].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[22].wrapping_sub(base)) >> (7 - 1);
    tmp |= (input[23].wrapping_sub(base)) << 1;
    tmp |= (input[24].wrapping_sub(base)) << 8;
    tmp |= (input[25].wrapping_sub(base)) << 15;
    tmp |= (input[26].wrapping_sub(base)) << 22;
    tmp |= (input[27].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[27].wrapping_sub(base)) >> (7 - 4);
    tmp |= (input[28].wrapping_sub(base)) << 4;
    tmp |= (input[29].wrapping_sub(base)) << 11;
    tmp |= (input[30].wrapping_sub(base)) << 18;
    tmp |= (input[31].wrapping_sub(base)) << 25;
    write_partial(output, out_off, tmp, 4);
    28
}

pub fn unpack7_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 127);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 7) & 127);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 14) & 127);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 21) & 127);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 3)) << (7 - 3);
    output[4] = base.wrapping_add(tmp);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 3) & 127);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 10) & 127);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 17) & 127);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 24) & 127);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (7 - 6);
    output[9] = base.wrapping_add(tmp);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 6) & 127);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 13) & 127);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 20) & 127);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (7 - 2);
    output[13] = base.wrapping_add(tmp);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 2) & 127);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 9) & 127);
    output[16] = base.wrapping_add((read_u32(input, in_off) >> 16) & 127);
    output[17] = base.wrapping_add((read_u32(input, in_off) >> 23) & 127);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 5)) << (7 - 5);
    output[18] = base.wrapping_add(tmp);
    output[19] = base.wrapping_add((read_u32(input, in_off) >> 5) & 127);
    output[20] = base.wrapping_add((read_u32(input, in_off) >> 12) & 127);
    output[21] = base.wrapping_add((read_u32(input, in_off) >> 19) & 127);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (7 - 1);
    output[22] = base.wrapping_add(tmp);
    output[23] = base.wrapping_add((read_u32(input, in_off) >> 1) & 127);
    output[24] = base.wrapping_add((read_u32(input, in_off) >> 8) & 127);
    output[25] = base.wrapping_add((read_u32(input, in_off) >> 15) & 127);
    output[26] = base.wrapping_add((read_u32(input, in_off) >> 22) & 127);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (7 - 4);
    output[27] = base.wrapping_add(tmp);
    output[28] = base.wrapping_add((read_u32(input, in_off) >> 4) & 127);
    output[29] = base.wrapping_add((read_u32(input, in_off) >> 11) & 127);
    output[30] = base.wrapping_add((read_u32(input, in_off) >> 18) & 127);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 25) & 127);
    28
}

pub fn pack8_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 8;
    tmp |= (input[2].wrapping_sub(base)) << 16;
    tmp |= (input[3].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) << 0;
    tmp |= (input[5].wrapping_sub(base)) << 8;
    tmp |= (input[6].wrapping_sub(base)) << 16;
    tmp |= (input[7].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) << 0;
    tmp |= (input[9].wrapping_sub(base)) << 8;
    tmp |= (input[10].wrapping_sub(base)) << 16;
    tmp |= (input[11].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) << 0;
    tmp |= (input[13].wrapping_sub(base)) << 8;
    tmp |= (input[14].wrapping_sub(base)) << 16;
    tmp |= (input[15].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[16].wrapping_sub(base)) << 0;
    tmp |= (input[17].wrapping_sub(base)) << 8;
    tmp |= (input[18].wrapping_sub(base)) << 16;
    tmp |= (input[19].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[20].wrapping_sub(base)) << 0;
    tmp |= (input[21].wrapping_sub(base)) << 8;
    tmp |= (input[22].wrapping_sub(base)) << 16;
    tmp |= (input[23].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[24].wrapping_sub(base)) << 0;
    tmp |= (input[25].wrapping_sub(base)) << 8;
    tmp |= (input[26].wrapping_sub(base)) << 16;
    tmp |= (input[27].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[28].wrapping_sub(base)) << 0;
    tmp |= (input[29].wrapping_sub(base)) << 8;
    tmp |= (input[30].wrapping_sub(base)) << 16;
    tmp |= (input[31].wrapping_sub(base)) << 24;
    write_partial(output, out_off, tmp, 4);
    32
}

pub fn unpack8_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 255);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 8) & 255);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 16) & 255);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 24) & 255);
    in_off += 4;
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 0) & 255);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 8) & 255);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 16) & 255);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 24) & 255);
    in_off += 4;
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 0) & 255);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 8) & 255);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 16) & 255);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 24) & 255);
    in_off += 4;
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 0) & 255);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 8) & 255);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 16) & 255);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 24) & 255);
    in_off += 4;
    output[16] = base.wrapping_add((read_u32(input, in_off) >> 0) & 255);
    output[17] = base.wrapping_add((read_u32(input, in_off) >> 8) & 255);
    output[18] = base.wrapping_add((read_u32(input, in_off) >> 16) & 255);
    output[19] = base.wrapping_add((read_u32(input, in_off) >> 24) & 255);
    in_off += 4;
    output[20] = base.wrapping_add((read_u32(input, in_off) >> 0) & 255);
    output[21] = base.wrapping_add((read_u32(input, in_off) >> 8) & 255);
    output[22] = base.wrapping_add((read_u32(input, in_off) >> 16) & 255);
    output[23] = base.wrapping_add((read_u32(input, in_off) >> 24) & 255);
    in_off += 4;
    output[24] = base.wrapping_add((read_u32(input, in_off) >> 0) & 255);
    output[25] = base.wrapping_add((read_u32(input, in_off) >> 8) & 255);
    output[26] = base.wrapping_add((read_u32(input, in_off) >> 16) & 255);
    output[27] = base.wrapping_add((read_u32(input, in_off) >> 24) & 255);
    in_off += 4;
    output[28] = base.wrapping_add((read_u32(input, in_off) >> 0) & 255);
    output[29] = base.wrapping_add((read_u32(input, in_off) >> 8) & 255);
    output[30] = base.wrapping_add((read_u32(input, in_off) >> 16) & 255);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 24) & 255);
    32
}

pub fn pack9_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 9;
    tmp |= (input[2].wrapping_sub(base)) << 18;
    tmp |= (input[3].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (9 - 4);
    tmp |= (input[4].wrapping_sub(base)) << 4;
    tmp |= (input[5].wrapping_sub(base)) << 13;
    tmp |= (input[6].wrapping_sub(base)) << 22;
    tmp |= (input[7].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (9 - 8);
    tmp |= (input[8].wrapping_sub(base)) << 8;
    tmp |= (input[9].wrapping_sub(base)) << 17;
    tmp |= (input[10].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (9 - 3);
    tmp |= (input[11].wrapping_sub(base)) << 3;
    tmp |= (input[12].wrapping_sub(base)) << 12;
    tmp |= (input[13].wrapping_sub(base)) << 21;
    tmp |= (input[14].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (9 - 7);
    tmp |= (input[15].wrapping_sub(base)) << 7;
    tmp |= (input[16].wrapping_sub(base)) << 16;
    tmp |= (input[17].wrapping_sub(base)) << 25;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[17].wrapping_sub(base)) >> (9 - 2);
    tmp |= (input[18].wrapping_sub(base)) << 2;
    tmp |= (input[19].wrapping_sub(base)) << 11;
    tmp |= (input[20].wrapping_sub(base)) << 20;
    tmp |= (input[21].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[21].wrapping_sub(base)) >> (9 - 6);
    tmp |= (input[22].wrapping_sub(base)) << 6;
    tmp |= (input[23].wrapping_sub(base)) << 15;
    tmp |= (input[24].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[24].wrapping_sub(base)) >> (9 - 1);
    tmp |= (input[25].wrapping_sub(base)) << 1;
    tmp |= (input[26].wrapping_sub(base)) << 10;
    tmp |= (input[27].wrapping_sub(base)) << 19;
    tmp |= (input[28].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[28].wrapping_sub(base)) >> (9 - 5);
    tmp |= (input[29].wrapping_sub(base)) << 5;
    tmp |= (input[30].wrapping_sub(base)) << 14;
    tmp |= (input[31].wrapping_sub(base)) << 23;
    write_partial(output, out_off, tmp, 4);
    36
}

pub fn unpack9_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 511);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 9) & 511);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 18) & 511);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (9 - 4);
    output[3] = base.wrapping_add(tmp);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 4) & 511);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 13) & 511);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 22) & 511);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (9 - 8);
    output[7] = base.wrapping_add(tmp);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 8) & 511);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 17) & 511);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 3)) << (9 - 3);
    output[10] = base.wrapping_add(tmp);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 3) & 511);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 12) & 511);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 21) & 511);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 7)) << (9 - 7);
    output[14] = base.wrapping_add(tmp);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 7) & 511);
    output[16] = base.wrapping_add((read_u32(input, in_off) >> 16) & 511);
    tmp = read_u32(input, in_off) >> 25;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (9 - 2);
    output[17] = base.wrapping_add(tmp);
    output[18] = base.wrapping_add((read_u32(input, in_off) >> 2) & 511);
    output[19] = base.wrapping_add((read_u32(input, in_off) >> 11) & 511);
    output[20] = base.wrapping_add((read_u32(input, in_off) >> 20) & 511);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (9 - 6);
    output[21] = base.wrapping_add(tmp);
    output[22] = base.wrapping_add((read_u32(input, in_off) >> 6) & 511);
    output[23] = base.wrapping_add((read_u32(input, in_off) >> 15) & 511);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (9 - 1);
    output[24] = base.wrapping_add(tmp);
    output[25] = base.wrapping_add((read_u32(input, in_off) >> 1) & 511);
    output[26] = base.wrapping_add((read_u32(input, in_off) >> 10) & 511);
    output[27] = base.wrapping_add((read_u32(input, in_off) >> 19) & 511);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 5)) << (9 - 5);
    output[28] = base.wrapping_add(tmp);
    output[29] = base.wrapping_add((read_u32(input, in_off) >> 5) & 511);
    output[30] = base.wrapping_add((read_u32(input, in_off) >> 14) & 511);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 23) & 511);
    36
}

pub fn pack10_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 10;
    tmp |= (input[2].wrapping_sub(base)) << 20;
    tmp |= (input[3].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (10 - 8);
    tmp |= (input[4].wrapping_sub(base)) << 8;
    tmp |= (input[5].wrapping_sub(base)) << 18;
    tmp |= (input[6].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (10 - 6);
    tmp |= (input[7].wrapping_sub(base)) << 6;
    tmp |= (input[8].wrapping_sub(base)) << 16;
    tmp |= (input[9].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (10 - 4);
    tmp |= (input[10].wrapping_sub(base)) << 4;
    tmp |= (input[11].wrapping_sub(base)) << 14;
    tmp |= (input[12].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (10 - 2);
    tmp |= (input[13].wrapping_sub(base)) << 2;
    tmp |= (input[14].wrapping_sub(base)) << 12;
    tmp |= (input[15].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[16].wrapping_sub(base)) << 0;
    tmp |= (input[17].wrapping_sub(base)) << 10;
    tmp |= (input[18].wrapping_sub(base)) << 20;
    tmp |= (input[19].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[19].wrapping_sub(base)) >> (10 - 8);
    tmp |= (input[20].wrapping_sub(base)) << 8;
    tmp |= (input[21].wrapping_sub(base)) << 18;
    tmp |= (input[22].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[22].wrapping_sub(base)) >> (10 - 6);
    tmp |= (input[23].wrapping_sub(base)) << 6;
    tmp |= (input[24].wrapping_sub(base)) << 16;
    tmp |= (input[25].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[25].wrapping_sub(base)) >> (10 - 4);
    tmp |= (input[26].wrapping_sub(base)) << 4;
    tmp |= (input[27].wrapping_sub(base)) << 14;
    tmp |= (input[28].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[28].wrapping_sub(base)) >> (10 - 2);
    tmp |= (input[29].wrapping_sub(base)) << 2;
    tmp |= (input[30].wrapping_sub(base)) << 12;
    tmp |= (input[31].wrapping_sub(base)) << 22;
    write_partial(output, out_off, tmp, 4);
    40
}

pub fn unpack10_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 1023);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 10) & 1023);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 20) & 1023);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (10 - 8);
    output[3] = base.wrapping_add(tmp);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 8) & 1023);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 18) & 1023);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (10 - 6);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 6) & 1023);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 16) & 1023);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (10 - 4);
    output[9] = base.wrapping_add(tmp);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 4) & 1023);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 14) & 1023);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (10 - 2);
    output[12] = base.wrapping_add(tmp);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 2) & 1023);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 12) & 1023);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 22) & 1023);
    in_off += 4;
    output[16] = base.wrapping_add((read_u32(input, in_off) >> 0) & 1023);
    output[17] = base.wrapping_add((read_u32(input, in_off) >> 10) & 1023);
    output[18] = base.wrapping_add((read_u32(input, in_off) >> 20) & 1023);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (10 - 8);
    output[19] = base.wrapping_add(tmp);
    output[20] = base.wrapping_add((read_u32(input, in_off) >> 8) & 1023);
    output[21] = base.wrapping_add((read_u32(input, in_off) >> 18) & 1023);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (10 - 6);
    output[22] = base.wrapping_add(tmp);
    output[23] = base.wrapping_add((read_u32(input, in_off) >> 6) & 1023);
    output[24] = base.wrapping_add((read_u32(input, in_off) >> 16) & 1023);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (10 - 4);
    output[25] = base.wrapping_add(tmp);
    output[26] = base.wrapping_add((read_u32(input, in_off) >> 4) & 1023);
    output[27] = base.wrapping_add((read_u32(input, in_off) >> 14) & 1023);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (10 - 2);
    output[28] = base.wrapping_add(tmp);
    output[29] = base.wrapping_add((read_u32(input, in_off) >> 2) & 1023);
    output[30] = base.wrapping_add((read_u32(input, in_off) >> 12) & 1023);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 22) & 1023);
    40
}

pub fn pack11_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 11;
    tmp |= (input[2].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (11 - 1);
    tmp |= (input[3].wrapping_sub(base)) << 1;
    tmp |= (input[4].wrapping_sub(base)) << 12;
    tmp |= (input[5].wrapping_sub(base)) << 23;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (11 - 2);
    tmp |= (input[6].wrapping_sub(base)) << 2;
    tmp |= (input[7].wrapping_sub(base)) << 13;
    tmp |= (input[8].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (11 - 3);
    tmp |= (input[9].wrapping_sub(base)) << 3;
    tmp |= (input[10].wrapping_sub(base)) << 14;
    tmp |= (input[11].wrapping_sub(base)) << 25;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (11 - 4);
    tmp |= (input[12].wrapping_sub(base)) << 4;
    tmp |= (input[13].wrapping_sub(base)) << 15;
    tmp |= (input[14].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (11 - 5);
    tmp |= (input[15].wrapping_sub(base)) << 5;
    tmp |= (input[16].wrapping_sub(base)) << 16;
    tmp |= (input[17].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[17].wrapping_sub(base)) >> (11 - 6);
    tmp |= (input[18].wrapping_sub(base)) << 6;
    tmp |= (input[19].wrapping_sub(base)) << 17;
    tmp |= (input[20].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[20].wrapping_sub(base)) >> (11 - 7);
    tmp |= (input[21].wrapping_sub(base)) << 7;
    tmp |= (input[22].wrapping_sub(base)) << 18;
    tmp |= (input[23].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[23].wrapping_sub(base)) >> (11 - 8);
    tmp |= (input[24].wrapping_sub(base)) << 8;
    tmp |= (input[25].wrapping_sub(base)) << 19;
    tmp |= (input[26].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[26].wrapping_sub(base)) >> (11 - 9);
    tmp |= (input[27].wrapping_sub(base)) << 9;
    tmp |= (input[28].wrapping_sub(base)) << 20;
    tmp |= (input[29].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[29].wrapping_sub(base)) >> (11 - 10);
    tmp |= (input[30].wrapping_sub(base)) << 10;
    tmp |= (input[31].wrapping_sub(base)) << 21;
    write_partial(output, out_off, tmp, 4);
    44
}

pub fn unpack11_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 2047);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 11) & 2047);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (11 - 1);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 1) & 2047);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 12) & 2047);
    tmp = read_u32(input, in_off) >> 23;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (11 - 2);
    output[5] = base.wrapping_add(tmp);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 2) & 2047);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 13) & 2047);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 3)) << (11 - 3);
    output[8] = base.wrapping_add(tmp);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 3) & 2047);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 14) & 2047);
    tmp = read_u32(input, in_off) >> 25;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (11 - 4);
    output[11] = base.wrapping_add(tmp);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 4) & 2047);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 15) & 2047);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 5)) << (11 - 5);
    output[14] = base.wrapping_add(tmp);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 5) & 2047);
    output[16] = base.wrapping_add((read_u32(input, in_off) >> 16) & 2047);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (11 - 6);
    output[17] = base.wrapping_add(tmp);
    output[18] = base.wrapping_add((read_u32(input, in_off) >> 6) & 2047);
    output[19] = base.wrapping_add((read_u32(input, in_off) >> 17) & 2047);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 7)) << (11 - 7);
    output[20] = base.wrapping_add(tmp);
    output[21] = base.wrapping_add((read_u32(input, in_off) >> 7) & 2047);
    output[22] = base.wrapping_add((read_u32(input, in_off) >> 18) & 2047);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (11 - 8);
    output[23] = base.wrapping_add(tmp);
    output[24] = base.wrapping_add((read_u32(input, in_off) >> 8) & 2047);
    output[25] = base.wrapping_add((read_u32(input, in_off) >> 19) & 2047);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 9)) << (11 - 9);
    output[26] = base.wrapping_add(tmp);
    output[27] = base.wrapping_add((read_u32(input, in_off) >> 9) & 2047);
    output[28] = base.wrapping_add((read_u32(input, in_off) >> 20) & 2047);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (11 - 10);
    output[29] = base.wrapping_add(tmp);
    output[30] = base.wrapping_add((read_u32(input, in_off) >> 10) & 2047);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 21) & 2047);
    44
}

pub fn pack12_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 12;
    tmp |= (input[2].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (12 - 4);
    tmp |= (input[3].wrapping_sub(base)) << 4;
    tmp |= (input[4].wrapping_sub(base)) << 16;
    tmp |= (input[5].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (12 - 8);
    tmp |= (input[6].wrapping_sub(base)) << 8;
    tmp |= (input[7].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) << 0;
    tmp |= (input[9].wrapping_sub(base)) << 12;
    tmp |= (input[10].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (12 - 4);
    tmp |= (input[11].wrapping_sub(base)) << 4;
    tmp |= (input[12].wrapping_sub(base)) << 16;
    tmp |= (input[13].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (12 - 8);
    tmp |= (input[14].wrapping_sub(base)) << 8;
    tmp |= (input[15].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[16].wrapping_sub(base)) << 0;
    tmp |= (input[17].wrapping_sub(base)) << 12;
    tmp |= (input[18].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[18].wrapping_sub(base)) >> (12 - 4);
    tmp |= (input[19].wrapping_sub(base)) << 4;
    tmp |= (input[20].wrapping_sub(base)) << 16;
    tmp |= (input[21].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[21].wrapping_sub(base)) >> (12 - 8);
    tmp |= (input[22].wrapping_sub(base)) << 8;
    tmp |= (input[23].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[24].wrapping_sub(base)) << 0;
    tmp |= (input[25].wrapping_sub(base)) << 12;
    tmp |= (input[26].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[26].wrapping_sub(base)) >> (12 - 4);
    tmp |= (input[27].wrapping_sub(base)) << 4;
    tmp |= (input[28].wrapping_sub(base)) << 16;
    tmp |= (input[29].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[29].wrapping_sub(base)) >> (12 - 8);
    tmp |= (input[30].wrapping_sub(base)) << 8;
    tmp |= (input[31].wrapping_sub(base)) << 20;
    write_partial(output, out_off, tmp, 4);
    48
}

pub fn unpack12_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 4095);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 12) & 4095);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (12 - 4);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 4) & 4095);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 16) & 4095);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (12 - 8);
    output[5] = base.wrapping_add(tmp);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 8) & 4095);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 20) & 4095);
    in_off += 4;
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 0) & 4095);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 12) & 4095);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (12 - 4);
    output[10] = base.wrapping_add(tmp);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 4) & 4095);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 16) & 4095);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (12 - 8);
    output[13] = base.wrapping_add(tmp);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 8) & 4095);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 20) & 4095);
    in_off += 4;
    output[16] = base.wrapping_add((read_u32(input, in_off) >> 0) & 4095);
    output[17] = base.wrapping_add((read_u32(input, in_off) >> 12) & 4095);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (12 - 4);
    output[18] = base.wrapping_add(tmp);
    output[19] = base.wrapping_add((read_u32(input, in_off) >> 4) & 4095);
    output[20] = base.wrapping_add((read_u32(input, in_off) >> 16) & 4095);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (12 - 8);
    output[21] = base.wrapping_add(tmp);
    output[22] = base.wrapping_add((read_u32(input, in_off) >> 8) & 4095);
    output[23] = base.wrapping_add((read_u32(input, in_off) >> 20) & 4095);
    in_off += 4;
    output[24] = base.wrapping_add((read_u32(input, in_off) >> 0) & 4095);
    output[25] = base.wrapping_add((read_u32(input, in_off) >> 12) & 4095);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (12 - 4);
    output[26] = base.wrapping_add(tmp);
    output[27] = base.wrapping_add((read_u32(input, in_off) >> 4) & 4095);
    output[28] = base.wrapping_add((read_u32(input, in_off) >> 16) & 4095);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (12 - 8);
    output[29] = base.wrapping_add(tmp);
    output[30] = base.wrapping_add((read_u32(input, in_off) >> 8) & 4095);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 20) & 4095);
    48
}

pub fn pack13_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 13;
    tmp |= (input[2].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (13 - 7);
    tmp |= (input[3].wrapping_sub(base)) << 7;
    tmp |= (input[4].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (13 - 1);
    tmp |= (input[5].wrapping_sub(base)) << 1;
    tmp |= (input[6].wrapping_sub(base)) << 14;
    tmp |= (input[7].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (13 - 8);
    tmp |= (input[8].wrapping_sub(base)) << 8;
    tmp |= (input[9].wrapping_sub(base)) << 21;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (13 - 2);
    tmp |= (input[10].wrapping_sub(base)) << 2;
    tmp |= (input[11].wrapping_sub(base)) << 15;
    tmp |= (input[12].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (13 - 9);
    tmp |= (input[13].wrapping_sub(base)) << 9;
    tmp |= (input[14].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (13 - 3);
    tmp |= (input[15].wrapping_sub(base)) << 3;
    tmp |= (input[16].wrapping_sub(base)) << 16;
    tmp |= (input[17].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[17].wrapping_sub(base)) >> (13 - 10);
    tmp |= (input[18].wrapping_sub(base)) << 10;
    tmp |= (input[19].wrapping_sub(base)) << 23;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[19].wrapping_sub(base)) >> (13 - 4);
    tmp |= (input[20].wrapping_sub(base)) << 4;
    tmp |= (input[21].wrapping_sub(base)) << 17;
    tmp |= (input[22].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[22].wrapping_sub(base)) >> (13 - 11);
    tmp |= (input[23].wrapping_sub(base)) << 11;
    tmp |= (input[24].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[24].wrapping_sub(base)) >> (13 - 5);
    tmp |= (input[25].wrapping_sub(base)) << 5;
    tmp |= (input[26].wrapping_sub(base)) << 18;
    tmp |= (input[27].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[27].wrapping_sub(base)) >> (13 - 12);
    tmp |= (input[28].wrapping_sub(base)) << 12;
    tmp |= (input[29].wrapping_sub(base)) << 25;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[29].wrapping_sub(base)) >> (13 - 6);
    tmp |= (input[30].wrapping_sub(base)) << 6;
    tmp |= (input[31].wrapping_sub(base)) << 19;
    write_partial(output, out_off, tmp, 4);
    52
}

pub fn unpack13_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 8191);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 13) & 8191);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 7)) << (13 - 7);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 7) & 8191);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (13 - 1);
    output[4] = base.wrapping_add(tmp);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 1) & 8191);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 14) & 8191);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (13 - 8);
    output[7] = base.wrapping_add(tmp);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 8) & 8191);
    tmp = read_u32(input, in_off) >> 21;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (13 - 2);
    output[9] = base.wrapping_add(tmp);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 2) & 8191);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 15) & 8191);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 9)) << (13 - 9);
    output[12] = base.wrapping_add(tmp);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 9) & 8191);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 3)) << (13 - 3);
    output[14] = base.wrapping_add(tmp);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 3) & 8191);
    output[16] = base.wrapping_add((read_u32(input, in_off) >> 16) & 8191);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (13 - 10);
    output[17] = base.wrapping_add(tmp);
    output[18] = base.wrapping_add((read_u32(input, in_off) >> 10) & 8191);
    tmp = read_u32(input, in_off) >> 23;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (13 - 4);
    output[19] = base.wrapping_add(tmp);
    output[20] = base.wrapping_add((read_u32(input, in_off) >> 4) & 8191);
    output[21] = base.wrapping_add((read_u32(input, in_off) >> 17) & 8191);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 11)) << (13 - 11);
    output[22] = base.wrapping_add(tmp);
    output[23] = base.wrapping_add((read_u32(input, in_off) >> 11) & 8191);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 5)) << (13 - 5);
    output[24] = base.wrapping_add(tmp);
    output[25] = base.wrapping_add((read_u32(input, in_off) >> 5) & 8191);
    output[26] = base.wrapping_add((read_u32(input, in_off) >> 18) & 8191);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (13 - 12);
    output[27] = base.wrapping_add(tmp);
    output[28] = base.wrapping_add((read_u32(input, in_off) >> 12) & 8191);
    tmp = read_u32(input, in_off) >> 25;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (13 - 6);
    output[29] = base.wrapping_add(tmp);
    output[30] = base.wrapping_add((read_u32(input, in_off) >> 6) & 8191);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 19) & 8191);
    52
}

pub fn pack14_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 14;
    tmp |= (input[2].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (14 - 10);
    tmp |= (input[3].wrapping_sub(base)) << 10;
    tmp |= (input[4].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (14 - 6);
    tmp |= (input[5].wrapping_sub(base)) << 6;
    tmp |= (input[6].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (14 - 2);
    tmp |= (input[7].wrapping_sub(base)) << 2;
    tmp |= (input[8].wrapping_sub(base)) << 16;
    tmp |= (input[9].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (14 - 12);
    tmp |= (input[10].wrapping_sub(base)) << 12;
    tmp |= (input[11].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (14 - 8);
    tmp |= (input[12].wrapping_sub(base)) << 8;
    tmp |= (input[13].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (14 - 4);
    tmp |= (input[14].wrapping_sub(base)) << 4;
    tmp |= (input[15].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[16].wrapping_sub(base)) << 0;
    tmp |= (input[17].wrapping_sub(base)) << 14;
    tmp |= (input[18].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[18].wrapping_sub(base)) >> (14 - 10);
    tmp |= (input[19].wrapping_sub(base)) << 10;
    tmp |= (input[20].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[20].wrapping_sub(base)) >> (14 - 6);
    tmp |= (input[21].wrapping_sub(base)) << 6;
    tmp |= (input[22].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[22].wrapping_sub(base)) >> (14 - 2);
    tmp |= (input[23].wrapping_sub(base)) << 2;
    tmp |= (input[24].wrapping_sub(base)) << 16;
    tmp |= (input[25].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[25].wrapping_sub(base)) >> (14 - 12);
    tmp |= (input[26].wrapping_sub(base)) << 12;
    tmp |= (input[27].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[27].wrapping_sub(base)) >> (14 - 8);
    tmp |= (input[28].wrapping_sub(base)) << 8;
    tmp |= (input[29].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[29].wrapping_sub(base)) >> (14 - 4);
    tmp |= (input[30].wrapping_sub(base)) << 4;
    tmp |= (input[31].wrapping_sub(base)) << 18;
    write_partial(output, out_off, tmp, 4);
    56
}

pub fn unpack14_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 16383);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 14) & 16383);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (14 - 10);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 10) & 16383);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (14 - 6);
    output[4] = base.wrapping_add(tmp);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 6) & 16383);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (14 - 2);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 2) & 16383);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 16) & 16383);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (14 - 12);
    output[9] = base.wrapping_add(tmp);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 12) & 16383);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (14 - 8);
    output[11] = base.wrapping_add(tmp);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 8) & 16383);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (14 - 4);
    output[13] = base.wrapping_add(tmp);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 4) & 16383);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 18) & 16383);
    in_off += 4;
    output[16] = base.wrapping_add((read_u32(input, in_off) >> 0) & 16383);
    output[17] = base.wrapping_add((read_u32(input, in_off) >> 14) & 16383);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (14 - 10);
    output[18] = base.wrapping_add(tmp);
    output[19] = base.wrapping_add((read_u32(input, in_off) >> 10) & 16383);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (14 - 6);
    output[20] = base.wrapping_add(tmp);
    output[21] = base.wrapping_add((read_u32(input, in_off) >> 6) & 16383);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (14 - 2);
    output[22] = base.wrapping_add(tmp);
    output[23] = base.wrapping_add((read_u32(input, in_off) >> 2) & 16383);
    output[24] = base.wrapping_add((read_u32(input, in_off) >> 16) & 16383);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (14 - 12);
    output[25] = base.wrapping_add(tmp);
    output[26] = base.wrapping_add((read_u32(input, in_off) >> 12) & 16383);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (14 - 8);
    output[27] = base.wrapping_add(tmp);
    output[28] = base.wrapping_add((read_u32(input, in_off) >> 8) & 16383);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (14 - 4);
    output[29] = base.wrapping_add(tmp);
    output[30] = base.wrapping_add((read_u32(input, in_off) >> 4) & 16383);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 18) & 16383);
    56
}

pub fn pack15_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 15;
    tmp |= (input[2].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (15 - 13);
    tmp |= (input[3].wrapping_sub(base)) << 13;
    tmp |= (input[4].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (15 - 11);
    tmp |= (input[5].wrapping_sub(base)) << 11;
    tmp |= (input[6].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (15 - 9);
    tmp |= (input[7].wrapping_sub(base)) << 9;
    tmp |= (input[8].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (15 - 7);
    tmp |= (input[9].wrapping_sub(base)) << 7;
    tmp |= (input[10].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (15 - 5);
    tmp |= (input[11].wrapping_sub(base)) << 5;
    tmp |= (input[12].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (15 - 3);
    tmp |= (input[13].wrapping_sub(base)) << 3;
    tmp |= (input[14].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (15 - 1);
    tmp |= (input[15].wrapping_sub(base)) << 1;
    tmp |= (input[16].wrapping_sub(base)) << 16;
    tmp |= (input[17].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[17].wrapping_sub(base)) >> (15 - 14);
    tmp |= (input[18].wrapping_sub(base)) << 14;
    tmp |= (input[19].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[19].wrapping_sub(base)) >> (15 - 12);
    tmp |= (input[20].wrapping_sub(base)) << 12;
    tmp |= (input[21].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[21].wrapping_sub(base)) >> (15 - 10);
    tmp |= (input[22].wrapping_sub(base)) << 10;
    tmp |= (input[23].wrapping_sub(base)) << 25;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[23].wrapping_sub(base)) >> (15 - 8);
    tmp |= (input[24].wrapping_sub(base)) << 8;
    tmp |= (input[25].wrapping_sub(base)) << 23;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[25].wrapping_sub(base)) >> (15 - 6);
    tmp |= (input[26].wrapping_sub(base)) << 6;
    tmp |= (input[27].wrapping_sub(base)) << 21;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[27].wrapping_sub(base)) >> (15 - 4);
    tmp |= (input[28].wrapping_sub(base)) << 4;
    tmp |= (input[29].wrapping_sub(base)) << 19;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[29].wrapping_sub(base)) >> (15 - 2);
    tmp |= (input[30].wrapping_sub(base)) << 2;
    tmp |= (input[31].wrapping_sub(base)) << 17;
    write_partial(output, out_off, tmp, 4);
    60
}

pub fn unpack15_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 32767);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 15) & 32767);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 13)) << (15 - 13);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 13) & 32767);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 11)) << (15 - 11);
    output[4] = base.wrapping_add(tmp);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 11) & 32767);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 9)) << (15 - 9);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 9) & 32767);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 7)) << (15 - 7);
    output[8] = base.wrapping_add(tmp);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 7) & 32767);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 5)) << (15 - 5);
    output[10] = base.wrapping_add(tmp);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 5) & 32767);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 3)) << (15 - 3);
    output[12] = base.wrapping_add(tmp);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 3) & 32767);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (15 - 1);
    output[14] = base.wrapping_add(tmp);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 1) & 32767);
    output[16] = base.wrapping_add((read_u32(input, in_off) >> 16) & 32767);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (15 - 14);
    output[17] = base.wrapping_add(tmp);
    output[18] = base.wrapping_add((read_u32(input, in_off) >> 14) & 32767);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (15 - 12);
    output[19] = base.wrapping_add(tmp);
    output[20] = base.wrapping_add((read_u32(input, in_off) >> 12) & 32767);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (15 - 10);
    output[21] = base.wrapping_add(tmp);
    output[22] = base.wrapping_add((read_u32(input, in_off) >> 10) & 32767);
    tmp = read_u32(input, in_off) >> 25;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (15 - 8);
    output[23] = base.wrapping_add(tmp);
    output[24] = base.wrapping_add((read_u32(input, in_off) >> 8) & 32767);
    tmp = read_u32(input, in_off) >> 23;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (15 - 6);
    output[25] = base.wrapping_add(tmp);
    output[26] = base.wrapping_add((read_u32(input, in_off) >> 6) & 32767);
    tmp = read_u32(input, in_off) >> 21;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (15 - 4);
    output[27] = base.wrapping_add(tmp);
    output[28] = base.wrapping_add((read_u32(input, in_off) >> 4) & 32767);
    tmp = read_u32(input, in_off) >> 19;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (15 - 2);
    output[29] = base.wrapping_add(tmp);
    output[30] = base.wrapping_add((read_u32(input, in_off) >> 2) & 32767);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 17) & 32767);
    60
}

pub fn pack16_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) << 0;
    tmp |= (input[3].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) << 0;
    tmp |= (input[5].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) << 0;
    tmp |= (input[7].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) << 0;
    tmp |= (input[9].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) << 0;
    tmp |= (input[11].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) << 0;
    tmp |= (input[13].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) << 0;
    tmp |= (input[15].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[16].wrapping_sub(base)) << 0;
    tmp |= (input[17].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[18].wrapping_sub(base)) << 0;
    tmp |= (input[19].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[20].wrapping_sub(base)) << 0;
    tmp |= (input[21].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[22].wrapping_sub(base)) << 0;
    tmp |= (input[23].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[24].wrapping_sub(base)) << 0;
    tmp |= (input[25].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[26].wrapping_sub(base)) << 0;
    tmp |= (input[27].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[28].wrapping_sub(base)) << 0;
    tmp |= (input[29].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[30].wrapping_sub(base)) << 0;
    tmp |= (input[31].wrapping_sub(base)) << 16;
    write_partial(output, out_off, tmp, 4);
    64
}

pub fn unpack16_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[16] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[17] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[18] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[19] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[20] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[21] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[22] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[23] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[24] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[25] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[26] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[27] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[28] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[29] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[30] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    64
}

pub fn pack17_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 17;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (17 - 2);
    tmp |= (input[2].wrapping_sub(base)) << 2;
    tmp |= (input[3].wrapping_sub(base)) << 19;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (17 - 4);
    tmp |= (input[4].wrapping_sub(base)) << 4;
    tmp |= (input[5].wrapping_sub(base)) << 21;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (17 - 6);
    tmp |= (input[6].wrapping_sub(base)) << 6;
    tmp |= (input[7].wrapping_sub(base)) << 23;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (17 - 8);
    tmp |= (input[8].wrapping_sub(base)) << 8;
    tmp |= (input[9].wrapping_sub(base)) << 25;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (17 - 10);
    tmp |= (input[10].wrapping_sub(base)) << 10;
    tmp |= (input[11].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (17 - 12);
    tmp |= (input[12].wrapping_sub(base)) << 12;
    tmp |= (input[13].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (17 - 14);
    tmp |= (input[14].wrapping_sub(base)) << 14;
    tmp |= (input[15].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[15].wrapping_sub(base)) >> (17 - 16);
    tmp |= (input[16].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[16].wrapping_sub(base)) >> (17 - 1);
    tmp |= (input[17].wrapping_sub(base)) << 1;
    tmp |= (input[18].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[18].wrapping_sub(base)) >> (17 - 3);
    tmp |= (input[19].wrapping_sub(base)) << 3;
    tmp |= (input[20].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[20].wrapping_sub(base)) >> (17 - 5);
    tmp |= (input[21].wrapping_sub(base)) << 5;
    tmp |= (input[22].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[22].wrapping_sub(base)) >> (17 - 7);
    tmp |= (input[23].wrapping_sub(base)) << 7;
    tmp |= (input[24].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[24].wrapping_sub(base)) >> (17 - 9);
    tmp |= (input[25].wrapping_sub(base)) << 9;
    tmp |= (input[26].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[26].wrapping_sub(base)) >> (17 - 11);
    tmp |= (input[27].wrapping_sub(base)) << 11;
    tmp |= (input[28].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[28].wrapping_sub(base)) >> (17 - 13);
    tmp |= (input[29].wrapping_sub(base)) << 13;
    tmp |= (input[30].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[30].wrapping_sub(base)) >> (17 - 15);
    tmp |= (input[31].wrapping_sub(base)) << 15;
    write_partial(output, out_off, tmp, 4);
    68
}

pub fn unpack17_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 131071);
    tmp = read_u32(input, in_off) >> 17;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (17 - 2);
    output[1] = base.wrapping_add(tmp);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 2) & 131071);
    tmp = read_u32(input, in_off) >> 19;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (17 - 4);
    output[3] = base.wrapping_add(tmp);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 4) & 131071);
    tmp = read_u32(input, in_off) >> 21;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (17 - 6);
    output[5] = base.wrapping_add(tmp);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 6) & 131071);
    tmp = read_u32(input, in_off) >> 23;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (17 - 8);
    output[7] = base.wrapping_add(tmp);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 8) & 131071);
    tmp = read_u32(input, in_off) >> 25;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (17 - 10);
    output[9] = base.wrapping_add(tmp);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 10) & 131071);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (17 - 12);
    output[11] = base.wrapping_add(tmp);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 12) & 131071);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (17 - 14);
    output[13] = base.wrapping_add(tmp);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 14) & 131071);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (17 - 16);
    output[15] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (17 - 1);
    output[16] = base.wrapping_add(tmp);
    output[17] = base.wrapping_add((read_u32(input, in_off) >> 1) & 131071);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 3)) << (17 - 3);
    output[18] = base.wrapping_add(tmp);
    output[19] = base.wrapping_add((read_u32(input, in_off) >> 3) & 131071);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 5)) << (17 - 5);
    output[20] = base.wrapping_add(tmp);
    output[21] = base.wrapping_add((read_u32(input, in_off) >> 5) & 131071);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 7)) << (17 - 7);
    output[22] = base.wrapping_add(tmp);
    output[23] = base.wrapping_add((read_u32(input, in_off) >> 7) & 131071);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 9)) << (17 - 9);
    output[24] = base.wrapping_add(tmp);
    output[25] = base.wrapping_add((read_u32(input, in_off) >> 9) & 131071);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 11)) << (17 - 11);
    output[26] = base.wrapping_add(tmp);
    output[27] = base.wrapping_add((read_u32(input, in_off) >> 11) & 131071);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 13)) << (17 - 13);
    output[28] = base.wrapping_add(tmp);
    output[29] = base.wrapping_add((read_u32(input, in_off) >> 13) & 131071);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 15)) << (17 - 15);
    output[30] = base.wrapping_add(tmp);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 15) & 131071);
    68
}

pub fn pack18_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (18 - 4);
    tmp |= (input[2].wrapping_sub(base)) << 4;
    tmp |= (input[3].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (18 - 8);
    tmp |= (input[4].wrapping_sub(base)) << 8;
    tmp |= (input[5].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (18 - 12);
    tmp |= (input[6].wrapping_sub(base)) << 12;
    tmp |= (input[7].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (18 - 16);
    tmp |= (input[8].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (18 - 2);
    tmp |= (input[9].wrapping_sub(base)) << 2;
    tmp |= (input[10].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (18 - 6);
    tmp |= (input[11].wrapping_sub(base)) << 6;
    tmp |= (input[12].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (18 - 10);
    tmp |= (input[13].wrapping_sub(base)) << 10;
    tmp |= (input[14].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (18 - 14);
    tmp |= (input[15].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[16].wrapping_sub(base)) << 0;
    tmp |= (input[17].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[17].wrapping_sub(base)) >> (18 - 4);
    tmp |= (input[18].wrapping_sub(base)) << 4;
    tmp |= (input[19].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[19].wrapping_sub(base)) >> (18 - 8);
    tmp |= (input[20].wrapping_sub(base)) << 8;
    tmp |= (input[21].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[21].wrapping_sub(base)) >> (18 - 12);
    tmp |= (input[22].wrapping_sub(base)) << 12;
    tmp |= (input[23].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[23].wrapping_sub(base)) >> (18 - 16);
    tmp |= (input[24].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[24].wrapping_sub(base)) >> (18 - 2);
    tmp |= (input[25].wrapping_sub(base)) << 2;
    tmp |= (input[26].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[26].wrapping_sub(base)) >> (18 - 6);
    tmp |= (input[27].wrapping_sub(base)) << 6;
    tmp |= (input[28].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[28].wrapping_sub(base)) >> (18 - 10);
    tmp |= (input[29].wrapping_sub(base)) << 10;
    tmp |= (input[30].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[30].wrapping_sub(base)) >> (18 - 14);
    tmp |= (input[31].wrapping_sub(base)) << 14;
    write_partial(output, out_off, tmp, 4);
    72
}

pub fn unpack18_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 262143);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (18 - 4);
    output[1] = base.wrapping_add(tmp);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 4) & 262143);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (18 - 8);
    output[3] = base.wrapping_add(tmp);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 8) & 262143);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (18 - 12);
    output[5] = base.wrapping_add(tmp);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 12) & 262143);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (18 - 16);
    output[7] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (18 - 2);
    output[8] = base.wrapping_add(tmp);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 2) & 262143);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (18 - 6);
    output[10] = base.wrapping_add(tmp);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 6) & 262143);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (18 - 10);
    output[12] = base.wrapping_add(tmp);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 10) & 262143);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (18 - 14);
    output[14] = base.wrapping_add(tmp);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 14) & 262143);
    in_off += 4;
    output[16] = base.wrapping_add((read_u32(input, in_off) >> 0) & 262143);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (18 - 4);
    output[17] = base.wrapping_add(tmp);
    output[18] = base.wrapping_add((read_u32(input, in_off) >> 4) & 262143);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (18 - 8);
    output[19] = base.wrapping_add(tmp);
    output[20] = base.wrapping_add((read_u32(input, in_off) >> 8) & 262143);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (18 - 12);
    output[21] = base.wrapping_add(tmp);
    output[22] = base.wrapping_add((read_u32(input, in_off) >> 12) & 262143);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (18 - 16);
    output[23] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (18 - 2);
    output[24] = base.wrapping_add(tmp);
    output[25] = base.wrapping_add((read_u32(input, in_off) >> 2) & 262143);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (18 - 6);
    output[26] = base.wrapping_add(tmp);
    output[27] = base.wrapping_add((read_u32(input, in_off) >> 6) & 262143);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (18 - 10);
    output[28] = base.wrapping_add(tmp);
    output[29] = base.wrapping_add((read_u32(input, in_off) >> 10) & 262143);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (18 - 14);
    output[30] = base.wrapping_add(tmp);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 14) & 262143);
    72
}

pub fn pack19_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 19;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (19 - 6);
    tmp |= (input[2].wrapping_sub(base)) << 6;
    tmp |= (input[3].wrapping_sub(base)) << 25;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (19 - 12);
    tmp |= (input[4].wrapping_sub(base)) << 12;
    tmp |= (input[5].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (19 - 18);
    tmp |= (input[6].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (19 - 5);
    tmp |= (input[7].wrapping_sub(base)) << 5;
    tmp |= (input[8].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (19 - 11);
    tmp |= (input[9].wrapping_sub(base)) << 11;
    tmp |= (input[10].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (19 - 17);
    tmp |= (input[11].wrapping_sub(base)) << 17;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (19 - 4);
    tmp |= (input[12].wrapping_sub(base)) << 4;
    tmp |= (input[13].wrapping_sub(base)) << 23;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (19 - 10);
    tmp |= (input[14].wrapping_sub(base)) << 10;
    tmp |= (input[15].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[15].wrapping_sub(base)) >> (19 - 16);
    tmp |= (input[16].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[16].wrapping_sub(base)) >> (19 - 3);
    tmp |= (input[17].wrapping_sub(base)) << 3;
    tmp |= (input[18].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[18].wrapping_sub(base)) >> (19 - 9);
    tmp |= (input[19].wrapping_sub(base)) << 9;
    tmp |= (input[20].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[20].wrapping_sub(base)) >> (19 - 15);
    tmp |= (input[21].wrapping_sub(base)) << 15;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[21].wrapping_sub(base)) >> (19 - 2);
    tmp |= (input[22].wrapping_sub(base)) << 2;
    tmp |= (input[23].wrapping_sub(base)) << 21;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[23].wrapping_sub(base)) >> (19 - 8);
    tmp |= (input[24].wrapping_sub(base)) << 8;
    tmp |= (input[25].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[25].wrapping_sub(base)) >> (19 - 14);
    tmp |= (input[26].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[26].wrapping_sub(base)) >> (19 - 1);
    tmp |= (input[27].wrapping_sub(base)) << 1;
    tmp |= (input[28].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[28].wrapping_sub(base)) >> (19 - 7);
    tmp |= (input[29].wrapping_sub(base)) << 7;
    tmp |= (input[30].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[30].wrapping_sub(base)) >> (19 - 13);
    tmp |= (input[31].wrapping_sub(base)) << 13;
    write_partial(output, out_off, tmp, 4);
    76
}

pub fn unpack19_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 524287);
    tmp = read_u32(input, in_off) >> 19;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (19 - 6);
    output[1] = base.wrapping_add(tmp);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 6) & 524287);
    tmp = read_u32(input, in_off) >> 25;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (19 - 12);
    output[3] = base.wrapping_add(tmp);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 12) & 524287);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 18)) << (19 - 18);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 5)) << (19 - 5);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 5) & 524287);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 11)) << (19 - 11);
    output[8] = base.wrapping_add(tmp);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 11) & 524287);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 17)) << (19 - 17);
    output[10] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 17;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (19 - 4);
    output[11] = base.wrapping_add(tmp);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 4) & 524287);
    tmp = read_u32(input, in_off) >> 23;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (19 - 10);
    output[13] = base.wrapping_add(tmp);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 10) & 524287);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (19 - 16);
    output[15] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 3)) << (19 - 3);
    output[16] = base.wrapping_add(tmp);
    output[17] = base.wrapping_add((read_u32(input, in_off) >> 3) & 524287);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 9)) << (19 - 9);
    output[18] = base.wrapping_add(tmp);
    output[19] = base.wrapping_add((read_u32(input, in_off) >> 9) & 524287);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 15)) << (19 - 15);
    output[20] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 15;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (19 - 2);
    output[21] = base.wrapping_add(tmp);
    output[22] = base.wrapping_add((read_u32(input, in_off) >> 2) & 524287);
    tmp = read_u32(input, in_off) >> 21;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (19 - 8);
    output[23] = base.wrapping_add(tmp);
    output[24] = base.wrapping_add((read_u32(input, in_off) >> 8) & 524287);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (19 - 14);
    output[25] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 14;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (19 - 1);
    output[26] = base.wrapping_add(tmp);
    output[27] = base.wrapping_add((read_u32(input, in_off) >> 1) & 524287);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 7)) << (19 - 7);
    output[28] = base.wrapping_add(tmp);
    output[29] = base.wrapping_add((read_u32(input, in_off) >> 7) & 524287);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 13)) << (19 - 13);
    output[30] = base.wrapping_add(tmp);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 13) & 524287);
    76
}

pub fn pack20_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (20 - 8);
    tmp |= (input[2].wrapping_sub(base)) << 8;
    tmp |= (input[3].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (20 - 16);
    tmp |= (input[4].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (20 - 4);
    tmp |= (input[5].wrapping_sub(base)) << 4;
    tmp |= (input[6].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (20 - 12);
    tmp |= (input[7].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) << 0;
    tmp |= (input[9].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (20 - 8);
    tmp |= (input[10].wrapping_sub(base)) << 8;
    tmp |= (input[11].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (20 - 16);
    tmp |= (input[12].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (20 - 4);
    tmp |= (input[13].wrapping_sub(base)) << 4;
    tmp |= (input[14].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (20 - 12);
    tmp |= (input[15].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[16].wrapping_sub(base)) << 0;
    tmp |= (input[17].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[17].wrapping_sub(base)) >> (20 - 8);
    tmp |= (input[18].wrapping_sub(base)) << 8;
    tmp |= (input[19].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[19].wrapping_sub(base)) >> (20 - 16);
    tmp |= (input[20].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[20].wrapping_sub(base)) >> (20 - 4);
    tmp |= (input[21].wrapping_sub(base)) << 4;
    tmp |= (input[22].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[22].wrapping_sub(base)) >> (20 - 12);
    tmp |= (input[23].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[24].wrapping_sub(base)) << 0;
    tmp |= (input[25].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[25].wrapping_sub(base)) >> (20 - 8);
    tmp |= (input[26].wrapping_sub(base)) << 8;
    tmp |= (input[27].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[27].wrapping_sub(base)) >> (20 - 16);
    tmp |= (input[28].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[28].wrapping_sub(base)) >> (20 - 4);
    tmp |= (input[29].wrapping_sub(base)) << 4;
    tmp |= (input[30].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[30].wrapping_sub(base)) >> (20 - 12);
    tmp |= (input[31].wrapping_sub(base)) << 12;
    write_partial(output, out_off, tmp, 4);
    80
}

pub fn unpack20_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 1048575);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (20 - 8);
    output[1] = base.wrapping_add(tmp);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 8) & 1048575);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (20 - 16);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (20 - 4);
    output[4] = base.wrapping_add(tmp);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 4) & 1048575);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (20 - 12);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 12) & 1048575);
    in_off += 4;
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 0) & 1048575);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (20 - 8);
    output[9] = base.wrapping_add(tmp);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 8) & 1048575);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (20 - 16);
    output[11] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (20 - 4);
    output[12] = base.wrapping_add(tmp);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 4) & 1048575);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (20 - 12);
    output[14] = base.wrapping_add(tmp);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 12) & 1048575);
    in_off += 4;
    output[16] = base.wrapping_add((read_u32(input, in_off) >> 0) & 1048575);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (20 - 8);
    output[17] = base.wrapping_add(tmp);
    output[18] = base.wrapping_add((read_u32(input, in_off) >> 8) & 1048575);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (20 - 16);
    output[19] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (20 - 4);
    output[20] = base.wrapping_add(tmp);
    output[21] = base.wrapping_add((read_u32(input, in_off) >> 4) & 1048575);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (20 - 12);
    output[22] = base.wrapping_add(tmp);
    output[23] = base.wrapping_add((read_u32(input, in_off) >> 12) & 1048575);
    in_off += 4;
    output[24] = base.wrapping_add((read_u32(input, in_off) >> 0) & 1048575);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (20 - 8);
    output[25] = base.wrapping_add(tmp);
    output[26] = base.wrapping_add((read_u32(input, in_off) >> 8) & 1048575);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (20 - 16);
    output[27] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (20 - 4);
    output[28] = base.wrapping_add(tmp);
    output[29] = base.wrapping_add((read_u32(input, in_off) >> 4) & 1048575);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (20 - 12);
    output[30] = base.wrapping_add(tmp);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 12) & 1048575);
    80
}

pub fn pack21_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 21;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (21 - 10);
    tmp |= (input[2].wrapping_sub(base)) << 10;
    tmp |= (input[3].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (21 - 20);
    tmp |= (input[4].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (21 - 9);
    tmp |= (input[5].wrapping_sub(base)) << 9;
    tmp |= (input[6].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (21 - 19);
    tmp |= (input[7].wrapping_sub(base)) << 19;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (21 - 8);
    tmp |= (input[8].wrapping_sub(base)) << 8;
    tmp |= (input[9].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (21 - 18);
    tmp |= (input[10].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (21 - 7);
    tmp |= (input[11].wrapping_sub(base)) << 7;
    tmp |= (input[12].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (21 - 17);
    tmp |= (input[13].wrapping_sub(base)) << 17;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (21 - 6);
    tmp |= (input[14].wrapping_sub(base)) << 6;
    tmp |= (input[15].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[15].wrapping_sub(base)) >> (21 - 16);
    tmp |= (input[16].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[16].wrapping_sub(base)) >> (21 - 5);
    tmp |= (input[17].wrapping_sub(base)) << 5;
    tmp |= (input[18].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[18].wrapping_sub(base)) >> (21 - 15);
    tmp |= (input[19].wrapping_sub(base)) << 15;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[19].wrapping_sub(base)) >> (21 - 4);
    tmp |= (input[20].wrapping_sub(base)) << 4;
    tmp |= (input[21].wrapping_sub(base)) << 25;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[21].wrapping_sub(base)) >> (21 - 14);
    tmp |= (input[22].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[22].wrapping_sub(base)) >> (21 - 3);
    tmp |= (input[23].wrapping_sub(base)) << 3;
    tmp |= (input[24].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[24].wrapping_sub(base)) >> (21 - 13);
    tmp |= (input[25].wrapping_sub(base)) << 13;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[25].wrapping_sub(base)) >> (21 - 2);
    tmp |= (input[26].wrapping_sub(base)) << 2;
    tmp |= (input[27].wrapping_sub(base)) << 23;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[27].wrapping_sub(base)) >> (21 - 12);
    tmp |= (input[28].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[28].wrapping_sub(base)) >> (21 - 1);
    tmp |= (input[29].wrapping_sub(base)) << 1;
    tmp |= (input[30].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[30].wrapping_sub(base)) >> (21 - 11);
    tmp |= (input[31].wrapping_sub(base)) << 11;
    write_partial(output, out_off, tmp, 4);
    84
}

pub fn unpack21_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 2097151);
    tmp = read_u32(input, in_off) >> 21;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (21 - 10);
    output[1] = base.wrapping_add(tmp);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 10) & 2097151);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (21 - 20);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 9)) << (21 - 9);
    output[4] = base.wrapping_add(tmp);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 9) & 2097151);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 19)) << (21 - 19);
    output[6] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 19;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (21 - 8);
    output[7] = base.wrapping_add(tmp);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 8) & 2097151);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 18)) << (21 - 18);
    output[9] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 7)) << (21 - 7);
    output[10] = base.wrapping_add(tmp);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 7) & 2097151);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 17)) << (21 - 17);
    output[12] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 17;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (21 - 6);
    output[13] = base.wrapping_add(tmp);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 6) & 2097151);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (21 - 16);
    output[15] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 5)) << (21 - 5);
    output[16] = base.wrapping_add(tmp);
    output[17] = base.wrapping_add((read_u32(input, in_off) >> 5) & 2097151);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 15)) << (21 - 15);
    output[18] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 15;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (21 - 4);
    output[19] = base.wrapping_add(tmp);
    output[20] = base.wrapping_add((read_u32(input, in_off) >> 4) & 2097151);
    tmp = read_u32(input, in_off) >> 25;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (21 - 14);
    output[21] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 14;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 3)) << (21 - 3);
    output[22] = base.wrapping_add(tmp);
    output[23] = base.wrapping_add((read_u32(input, in_off) >> 3) & 2097151);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 13)) << (21 - 13);
    output[24] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 13;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (21 - 2);
    output[25] = base.wrapping_add(tmp);
    output[26] = base.wrapping_add((read_u32(input, in_off) >> 2) & 2097151);
    tmp = read_u32(input, in_off) >> 23;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (21 - 12);
    output[27] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (21 - 1);
    output[28] = base.wrapping_add(tmp);
    output[29] = base.wrapping_add((read_u32(input, in_off) >> 1) & 2097151);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 11)) << (21 - 11);
    output[30] = base.wrapping_add(tmp);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 11) & 2097151);
    84
}

pub fn pack22_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (22 - 12);
    tmp |= (input[2].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (22 - 2);
    tmp |= (input[3].wrapping_sub(base)) << 2;
    tmp |= (input[4].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (22 - 14);
    tmp |= (input[5].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (22 - 4);
    tmp |= (input[6].wrapping_sub(base)) << 4;
    tmp |= (input[7].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (22 - 16);
    tmp |= (input[8].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (22 - 6);
    tmp |= (input[9].wrapping_sub(base)) << 6;
    tmp |= (input[10].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (22 - 18);
    tmp |= (input[11].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (22 - 8);
    tmp |= (input[12].wrapping_sub(base)) << 8;
    tmp |= (input[13].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (22 - 20);
    tmp |= (input[14].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (22 - 10);
    tmp |= (input[15].wrapping_sub(base)) << 10;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[16].wrapping_sub(base)) << 0;
    tmp |= (input[17].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[17].wrapping_sub(base)) >> (22 - 12);
    tmp |= (input[18].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[18].wrapping_sub(base)) >> (22 - 2);
    tmp |= (input[19].wrapping_sub(base)) << 2;
    tmp |= (input[20].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[20].wrapping_sub(base)) >> (22 - 14);
    tmp |= (input[21].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[21].wrapping_sub(base)) >> (22 - 4);
    tmp |= (input[22].wrapping_sub(base)) << 4;
    tmp |= (input[23].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[23].wrapping_sub(base)) >> (22 - 16);
    tmp |= (input[24].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[24].wrapping_sub(base)) >> (22 - 6);
    tmp |= (input[25].wrapping_sub(base)) << 6;
    tmp |= (input[26].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[26].wrapping_sub(base)) >> (22 - 18);
    tmp |= (input[27].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[27].wrapping_sub(base)) >> (22 - 8);
    tmp |= (input[28].wrapping_sub(base)) << 8;
    tmp |= (input[29].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[29].wrapping_sub(base)) >> (22 - 20);
    tmp |= (input[30].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[30].wrapping_sub(base)) >> (22 - 10);
    tmp |= (input[31].wrapping_sub(base)) << 10;
    write_partial(output, out_off, tmp, 4);
    88
}

pub fn unpack22_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 4194303);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (22 - 12);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (22 - 2);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 2) & 4194303);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (22 - 14);
    output[4] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 14;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (22 - 4);
    output[5] = base.wrapping_add(tmp);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 4) & 4194303);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (22 - 16);
    output[7] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (22 - 6);
    output[8] = base.wrapping_add(tmp);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 6) & 4194303);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 18)) << (22 - 18);
    output[10] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (22 - 8);
    output[11] = base.wrapping_add(tmp);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 8) & 4194303);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (22 - 20);
    output[13] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (22 - 10);
    output[14] = base.wrapping_add(tmp);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 10) & 4194303);
    in_off += 4;
    output[16] = base.wrapping_add((read_u32(input, in_off) >> 0) & 4194303);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (22 - 12);
    output[17] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (22 - 2);
    output[18] = base.wrapping_add(tmp);
    output[19] = base.wrapping_add((read_u32(input, in_off) >> 2) & 4194303);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (22 - 14);
    output[20] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 14;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (22 - 4);
    output[21] = base.wrapping_add(tmp);
    output[22] = base.wrapping_add((read_u32(input, in_off) >> 4) & 4194303);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (22 - 16);
    output[23] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (22 - 6);
    output[24] = base.wrapping_add(tmp);
    output[25] = base.wrapping_add((read_u32(input, in_off) >> 6) & 4194303);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 18)) << (22 - 18);
    output[26] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (22 - 8);
    output[27] = base.wrapping_add(tmp);
    output[28] = base.wrapping_add((read_u32(input, in_off) >> 8) & 4194303);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (22 - 20);
    output[29] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (22 - 10);
    output[30] = base.wrapping_add(tmp);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 10) & 4194303);
    88
}

pub fn pack23_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 23;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (23 - 14);
    tmp |= (input[2].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (23 - 5);
    tmp |= (input[3].wrapping_sub(base)) << 5;
    tmp |= (input[4].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (23 - 19);
    tmp |= (input[5].wrapping_sub(base)) << 19;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (23 - 10);
    tmp |= (input[6].wrapping_sub(base)) << 10;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (23 - 1);
    tmp |= (input[7].wrapping_sub(base)) << 1;
    tmp |= (input[8].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (23 - 15);
    tmp |= (input[9].wrapping_sub(base)) << 15;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (23 - 6);
    tmp |= (input[10].wrapping_sub(base)) << 6;
    tmp |= (input[11].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (23 - 20);
    tmp |= (input[12].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (23 - 11);
    tmp |= (input[13].wrapping_sub(base)) << 11;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (23 - 2);
    tmp |= (input[14].wrapping_sub(base)) << 2;
    tmp |= (input[15].wrapping_sub(base)) << 25;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[15].wrapping_sub(base)) >> (23 - 16);
    tmp |= (input[16].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[16].wrapping_sub(base)) >> (23 - 7);
    tmp |= (input[17].wrapping_sub(base)) << 7;
    tmp |= (input[18].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[18].wrapping_sub(base)) >> (23 - 21);
    tmp |= (input[19].wrapping_sub(base)) << 21;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[19].wrapping_sub(base)) >> (23 - 12);
    tmp |= (input[20].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[20].wrapping_sub(base)) >> (23 - 3);
    tmp |= (input[21].wrapping_sub(base)) << 3;
    tmp |= (input[22].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[22].wrapping_sub(base)) >> (23 - 17);
    tmp |= (input[23].wrapping_sub(base)) << 17;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[23].wrapping_sub(base)) >> (23 - 8);
    tmp |= (input[24].wrapping_sub(base)) << 8;
    tmp |= (input[25].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[25].wrapping_sub(base)) >> (23 - 22);
    tmp |= (input[26].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[26].wrapping_sub(base)) >> (23 - 13);
    tmp |= (input[27].wrapping_sub(base)) << 13;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[27].wrapping_sub(base)) >> (23 - 4);
    tmp |= (input[28].wrapping_sub(base)) << 4;
    tmp |= (input[29].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[29].wrapping_sub(base)) >> (23 - 18);
    tmp |= (input[30].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[30].wrapping_sub(base)) >> (23 - 9);
    tmp |= (input[31].wrapping_sub(base)) << 9;
    write_partial(output, out_off, tmp, 4);
    92
}

pub fn unpack23_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 8388607);
    tmp = read_u32(input, in_off) >> 23;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (23 - 14);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 14;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 5)) << (23 - 5);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 5) & 8388607);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 19)) << (23 - 19);
    output[4] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 19;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (23 - 10);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 10;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (23 - 1);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 1) & 8388607);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 15)) << (23 - 15);
    output[8] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 15;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (23 - 6);
    output[9] = base.wrapping_add(tmp);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 6) & 8388607);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (23 - 20);
    output[11] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 11)) << (23 - 11);
    output[12] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 11;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (23 - 2);
    output[13] = base.wrapping_add(tmp);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 2) & 8388607);
    tmp = read_u32(input, in_off) >> 25;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (23 - 16);
    output[15] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 7)) << (23 - 7);
    output[16] = base.wrapping_add(tmp);
    output[17] = base.wrapping_add((read_u32(input, in_off) >> 7) & 8388607);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 21)) << (23 - 21);
    output[18] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 21;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (23 - 12);
    output[19] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 3)) << (23 - 3);
    output[20] = base.wrapping_add(tmp);
    output[21] = base.wrapping_add((read_u32(input, in_off) >> 3) & 8388607);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 17)) << (23 - 17);
    output[22] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 17;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (23 - 8);
    output[23] = base.wrapping_add(tmp);
    output[24] = base.wrapping_add((read_u32(input, in_off) >> 8) & 8388607);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 22)) << (23 - 22);
    output[25] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 13)) << (23 - 13);
    output[26] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 13;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (23 - 4);
    output[27] = base.wrapping_add(tmp);
    output[28] = base.wrapping_add((read_u32(input, in_off) >> 4) & 8388607);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 18)) << (23 - 18);
    output[29] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 9)) << (23 - 9);
    output[30] = base.wrapping_add(tmp);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 9) & 8388607);
    92
}

pub fn pack24_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (24 - 16);
    tmp |= (input[2].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (24 - 8);
    tmp |= (input[3].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) << 0;
    tmp |= (input[5].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (24 - 16);
    tmp |= (input[6].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (24 - 8);
    tmp |= (input[7].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) << 0;
    tmp |= (input[9].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (24 - 16);
    tmp |= (input[10].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (24 - 8);
    tmp |= (input[11].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) << 0;
    tmp |= (input[13].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (24 - 16);
    tmp |= (input[14].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (24 - 8);
    tmp |= (input[15].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[16].wrapping_sub(base)) << 0;
    tmp |= (input[17].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[17].wrapping_sub(base)) >> (24 - 16);
    tmp |= (input[18].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[18].wrapping_sub(base)) >> (24 - 8);
    tmp |= (input[19].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[20].wrapping_sub(base)) << 0;
    tmp |= (input[21].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[21].wrapping_sub(base)) >> (24 - 16);
    tmp |= (input[22].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[22].wrapping_sub(base)) >> (24 - 8);
    tmp |= (input[23].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[24].wrapping_sub(base)) << 0;
    tmp |= (input[25].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[25].wrapping_sub(base)) >> (24 - 16);
    tmp |= (input[26].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[26].wrapping_sub(base)) >> (24 - 8);
    tmp |= (input[27].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[28].wrapping_sub(base)) << 0;
    tmp |= (input[29].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[29].wrapping_sub(base)) >> (24 - 16);
    tmp |= (input[30].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[30].wrapping_sub(base)) >> (24 - 8);
    tmp |= (input[31].wrapping_sub(base)) << 8;
    write_partial(output, out_off, tmp, 4);
    96
}

pub fn unpack24_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 16777215);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (24 - 16);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (24 - 8);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 8) & 16777215);
    in_off += 4;
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 0) & 16777215);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (24 - 16);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (24 - 8);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 8) & 16777215);
    in_off += 4;
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 0) & 16777215);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (24 - 16);
    output[9] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (24 - 8);
    output[10] = base.wrapping_add(tmp);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 8) & 16777215);
    in_off += 4;
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 0) & 16777215);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (24 - 16);
    output[13] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (24 - 8);
    output[14] = base.wrapping_add(tmp);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 8) & 16777215);
    in_off += 4;
    output[16] = base.wrapping_add((read_u32(input, in_off) >> 0) & 16777215);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (24 - 16);
    output[17] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (24 - 8);
    output[18] = base.wrapping_add(tmp);
    output[19] = base.wrapping_add((read_u32(input, in_off) >> 8) & 16777215);
    in_off += 4;
    output[20] = base.wrapping_add((read_u32(input, in_off) >> 0) & 16777215);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (24 - 16);
    output[21] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (24 - 8);
    output[22] = base.wrapping_add(tmp);
    output[23] = base.wrapping_add((read_u32(input, in_off) >> 8) & 16777215);
    in_off += 4;
    output[24] = base.wrapping_add((read_u32(input, in_off) >> 0) & 16777215);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (24 - 16);
    output[25] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (24 - 8);
    output[26] = base.wrapping_add(tmp);
    output[27] = base.wrapping_add((read_u32(input, in_off) >> 8) & 16777215);
    in_off += 4;
    output[28] = base.wrapping_add((read_u32(input, in_off) >> 0) & 16777215);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (24 - 16);
    output[29] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (24 - 8);
    output[30] = base.wrapping_add(tmp);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 8) & 16777215);
    96
}

pub fn pack25_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 25;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (25 - 18);
    tmp |= (input[2].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (25 - 11);
    tmp |= (input[3].wrapping_sub(base)) << 11;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (25 - 4);
    tmp |= (input[4].wrapping_sub(base)) << 4;
    tmp |= (input[5].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (25 - 22);
    tmp |= (input[6].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (25 - 15);
    tmp |= (input[7].wrapping_sub(base)) << 15;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (25 - 8);
    tmp |= (input[8].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (25 - 1);
    tmp |= (input[9].wrapping_sub(base)) << 1;
    tmp |= (input[10].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (25 - 19);
    tmp |= (input[11].wrapping_sub(base)) << 19;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (25 - 12);
    tmp |= (input[12].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (25 - 5);
    tmp |= (input[13].wrapping_sub(base)) << 5;
    tmp |= (input[14].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (25 - 23);
    tmp |= (input[15].wrapping_sub(base)) << 23;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[15].wrapping_sub(base)) >> (25 - 16);
    tmp |= (input[16].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[16].wrapping_sub(base)) >> (25 - 9);
    tmp |= (input[17].wrapping_sub(base)) << 9;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[17].wrapping_sub(base)) >> (25 - 2);
    tmp |= (input[18].wrapping_sub(base)) << 2;
    tmp |= (input[19].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[19].wrapping_sub(base)) >> (25 - 20);
    tmp |= (input[20].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[20].wrapping_sub(base)) >> (25 - 13);
    tmp |= (input[21].wrapping_sub(base)) << 13;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[21].wrapping_sub(base)) >> (25 - 6);
    tmp |= (input[22].wrapping_sub(base)) << 6;
    tmp |= (input[23].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[23].wrapping_sub(base)) >> (25 - 24);
    tmp |= (input[24].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[24].wrapping_sub(base)) >> (25 - 17);
    tmp |= (input[25].wrapping_sub(base)) << 17;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[25].wrapping_sub(base)) >> (25 - 10);
    tmp |= (input[26].wrapping_sub(base)) << 10;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[26].wrapping_sub(base)) >> (25 - 3);
    tmp |= (input[27].wrapping_sub(base)) << 3;
    tmp |= (input[28].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[28].wrapping_sub(base)) >> (25 - 21);
    tmp |= (input[29].wrapping_sub(base)) << 21;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[29].wrapping_sub(base)) >> (25 - 14);
    tmp |= (input[30].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[30].wrapping_sub(base)) >> (25 - 7);
    tmp |= (input[31].wrapping_sub(base)) << 7;
    write_partial(output, out_off, tmp, 4);
    100
}

pub fn unpack25_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 33554431);
    tmp = read_u32(input, in_off) >> 25;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 18)) << (25 - 18);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 11)) << (25 - 11);
    output[2] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 11;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (25 - 4);
    output[3] = base.wrapping_add(tmp);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 4) & 33554431);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 22)) << (25 - 22);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 15)) << (25 - 15);
    output[6] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 15;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (25 - 8);
    output[7] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 8;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (25 - 1);
    output[8] = base.wrapping_add(tmp);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 1) & 33554431);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 19)) << (25 - 19);
    output[10] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 19;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (25 - 12);
    output[11] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 5)) << (25 - 5);
    output[12] = base.wrapping_add(tmp);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 5) & 33554431);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 23)) << (25 - 23);
    output[14] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 23;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (25 - 16);
    output[15] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 9)) << (25 - 9);
    output[16] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 9;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (25 - 2);
    output[17] = base.wrapping_add(tmp);
    output[18] = base.wrapping_add((read_u32(input, in_off) >> 2) & 33554431);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (25 - 20);
    output[19] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 13)) << (25 - 13);
    output[20] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 13;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (25 - 6);
    output[21] = base.wrapping_add(tmp);
    output[22] = base.wrapping_add((read_u32(input, in_off) >> 6) & 33554431);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 24)) << (25 - 24);
    output[23] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 17)) << (25 - 17);
    output[24] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 17;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (25 - 10);
    output[25] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 10;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 3)) << (25 - 3);
    output[26] = base.wrapping_add(tmp);
    output[27] = base.wrapping_add((read_u32(input, in_off) >> 3) & 33554431);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 21)) << (25 - 21);
    output[28] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 21;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (25 - 14);
    output[29] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 14;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 7)) << (25 - 7);
    output[30] = base.wrapping_add(tmp);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 7) & 33554431);
    100
}

pub fn pack26_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (26 - 20);
    tmp |= (input[2].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (26 - 14);
    tmp |= (input[3].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (26 - 8);
    tmp |= (input[4].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (26 - 2);
    tmp |= (input[5].wrapping_sub(base)) << 2;
    tmp |= (input[6].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (26 - 22);
    tmp |= (input[7].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (26 - 16);
    tmp |= (input[8].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (26 - 10);
    tmp |= (input[9].wrapping_sub(base)) << 10;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (26 - 4);
    tmp |= (input[10].wrapping_sub(base)) << 4;
    tmp |= (input[11].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (26 - 24);
    tmp |= (input[12].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (26 - 18);
    tmp |= (input[13].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (26 - 12);
    tmp |= (input[14].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (26 - 6);
    tmp |= (input[15].wrapping_sub(base)) << 6;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[16].wrapping_sub(base)) << 0;
    tmp |= (input[17].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[17].wrapping_sub(base)) >> (26 - 20);
    tmp |= (input[18].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[18].wrapping_sub(base)) >> (26 - 14);
    tmp |= (input[19].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[19].wrapping_sub(base)) >> (26 - 8);
    tmp |= (input[20].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[20].wrapping_sub(base)) >> (26 - 2);
    tmp |= (input[21].wrapping_sub(base)) << 2;
    tmp |= (input[22].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[22].wrapping_sub(base)) >> (26 - 22);
    tmp |= (input[23].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[23].wrapping_sub(base)) >> (26 - 16);
    tmp |= (input[24].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[24].wrapping_sub(base)) >> (26 - 10);
    tmp |= (input[25].wrapping_sub(base)) << 10;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[25].wrapping_sub(base)) >> (26 - 4);
    tmp |= (input[26].wrapping_sub(base)) << 4;
    tmp |= (input[27].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[27].wrapping_sub(base)) >> (26 - 24);
    tmp |= (input[28].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[28].wrapping_sub(base)) >> (26 - 18);
    tmp |= (input[29].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[29].wrapping_sub(base)) >> (26 - 12);
    tmp |= (input[30].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[30].wrapping_sub(base)) >> (26 - 6);
    tmp |= (input[31].wrapping_sub(base)) << 6;
    write_partial(output, out_off, tmp, 4);
    104
}

pub fn unpack26_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 67108863);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (26 - 20);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (26 - 14);
    output[2] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 14;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (26 - 8);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 8;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (26 - 2);
    output[4] = base.wrapping_add(tmp);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 2) & 67108863);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 22)) << (26 - 22);
    output[6] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (26 - 16);
    output[7] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (26 - 10);
    output[8] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 10;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (26 - 4);
    output[9] = base.wrapping_add(tmp);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 4) & 67108863);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 24)) << (26 - 24);
    output[11] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 18)) << (26 - 18);
    output[12] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (26 - 12);
    output[13] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (26 - 6);
    output[14] = base.wrapping_add(tmp);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 6) & 67108863);
    in_off += 4;
    output[16] = base.wrapping_add((read_u32(input, in_off) >> 0) & 67108863);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (26 - 20);
    output[17] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (26 - 14);
    output[18] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 14;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (26 - 8);
    output[19] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 8;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (26 - 2);
    output[20] = base.wrapping_add(tmp);
    output[21] = base.wrapping_add((read_u32(input, in_off) >> 2) & 67108863);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 22)) << (26 - 22);
    output[22] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (26 - 16);
    output[23] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (26 - 10);
    output[24] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 10;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (26 - 4);
    output[25] = base.wrapping_add(tmp);
    output[26] = base.wrapping_add((read_u32(input, in_off) >> 4) & 67108863);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 24)) << (26 - 24);
    output[27] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 18)) << (26 - 18);
    output[28] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (26 - 12);
    output[29] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (26 - 6);
    output[30] = base.wrapping_add(tmp);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 6) & 67108863);
    104
}

pub fn pack27_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (27 - 22);
    tmp |= (input[2].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (27 - 17);
    tmp |= (input[3].wrapping_sub(base)) << 17;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (27 - 12);
    tmp |= (input[4].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (27 - 7);
    tmp |= (input[5].wrapping_sub(base)) << 7;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (27 - 2);
    tmp |= (input[6].wrapping_sub(base)) << 2;
    tmp |= (input[7].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (27 - 24);
    tmp |= (input[8].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (27 - 19);
    tmp |= (input[9].wrapping_sub(base)) << 19;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (27 - 14);
    tmp |= (input[10].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (27 - 9);
    tmp |= (input[11].wrapping_sub(base)) << 9;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (27 - 4);
    tmp |= (input[12].wrapping_sub(base)) << 4;
    tmp |= (input[13].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (27 - 26);
    tmp |= (input[14].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (27 - 21);
    tmp |= (input[15].wrapping_sub(base)) << 21;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[15].wrapping_sub(base)) >> (27 - 16);
    tmp |= (input[16].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[16].wrapping_sub(base)) >> (27 - 11);
    tmp |= (input[17].wrapping_sub(base)) << 11;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[17].wrapping_sub(base)) >> (27 - 6);
    tmp |= (input[18].wrapping_sub(base)) << 6;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[18].wrapping_sub(base)) >> (27 - 1);
    tmp |= (input[19].wrapping_sub(base)) << 1;
    tmp |= (input[20].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[20].wrapping_sub(base)) >> (27 - 23);
    tmp |= (input[21].wrapping_sub(base)) << 23;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[21].wrapping_sub(base)) >> (27 - 18);
    tmp |= (input[22].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[22].wrapping_sub(base)) >> (27 - 13);
    tmp |= (input[23].wrapping_sub(base)) << 13;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[23].wrapping_sub(base)) >> (27 - 8);
    tmp |= (input[24].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[24].wrapping_sub(base)) >> (27 - 3);
    tmp |= (input[25].wrapping_sub(base)) << 3;
    tmp |= (input[26].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[26].wrapping_sub(base)) >> (27 - 25);
    tmp |= (input[27].wrapping_sub(base)) << 25;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[27].wrapping_sub(base)) >> (27 - 20);
    tmp |= (input[28].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[28].wrapping_sub(base)) >> (27 - 15);
    tmp |= (input[29].wrapping_sub(base)) << 15;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[29].wrapping_sub(base)) >> (27 - 10);
    tmp |= (input[30].wrapping_sub(base)) << 10;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[30].wrapping_sub(base)) >> (27 - 5);
    tmp |= (input[31].wrapping_sub(base)) << 5;
    write_partial(output, out_off, tmp, 4);
    108
}

pub fn unpack27_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 134217727);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 22)) << (27 - 22);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 17)) << (27 - 17);
    output[2] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 17;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (27 - 12);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 7)) << (27 - 7);
    output[4] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 7;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (27 - 2);
    output[5] = base.wrapping_add(tmp);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 2) & 134217727);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 24)) << (27 - 24);
    output[7] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 19)) << (27 - 19);
    output[8] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 19;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (27 - 14);
    output[9] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 14;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 9)) << (27 - 9);
    output[10] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 9;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (27 - 4);
    output[11] = base.wrapping_add(tmp);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 4) & 134217727);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 26)) << (27 - 26);
    output[13] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 21)) << (27 - 21);
    output[14] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 21;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (27 - 16);
    output[15] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 11)) << (27 - 11);
    output[16] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 11;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (27 - 6);
    output[17] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 6;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (27 - 1);
    output[18] = base.wrapping_add(tmp);
    output[19] = base.wrapping_add((read_u32(input, in_off) >> 1) & 134217727);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 23)) << (27 - 23);
    output[20] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 23;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 18)) << (27 - 18);
    output[21] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 13)) << (27 - 13);
    output[22] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 13;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (27 - 8);
    output[23] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 8;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 3)) << (27 - 3);
    output[24] = base.wrapping_add(tmp);
    output[25] = base.wrapping_add((read_u32(input, in_off) >> 3) & 134217727);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 25)) << (27 - 25);
    output[26] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 25;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (27 - 20);
    output[27] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 15)) << (27 - 15);
    output[28] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 15;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (27 - 10);
    output[29] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 10;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 5)) << (27 - 5);
    output[30] = base.wrapping_add(tmp);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 5) & 134217727);
    108
}

pub fn pack28_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (28 - 24);
    tmp |= (input[2].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (28 - 20);
    tmp |= (input[3].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (28 - 16);
    tmp |= (input[4].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (28 - 12);
    tmp |= (input[5].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (28 - 8);
    tmp |= (input[6].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (28 - 4);
    tmp |= (input[7].wrapping_sub(base)) << 4;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) << 0;
    tmp |= (input[9].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (28 - 24);
    tmp |= (input[10].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (28 - 20);
    tmp |= (input[11].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (28 - 16);
    tmp |= (input[12].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (28 - 12);
    tmp |= (input[13].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (28 - 8);
    tmp |= (input[14].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (28 - 4);
    tmp |= (input[15].wrapping_sub(base)) << 4;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[16].wrapping_sub(base)) << 0;
    tmp |= (input[17].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[17].wrapping_sub(base)) >> (28 - 24);
    tmp |= (input[18].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[18].wrapping_sub(base)) >> (28 - 20);
    tmp |= (input[19].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[19].wrapping_sub(base)) >> (28 - 16);
    tmp |= (input[20].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[20].wrapping_sub(base)) >> (28 - 12);
    tmp |= (input[21].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[21].wrapping_sub(base)) >> (28 - 8);
    tmp |= (input[22].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[22].wrapping_sub(base)) >> (28 - 4);
    tmp |= (input[23].wrapping_sub(base)) << 4;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[24].wrapping_sub(base)) << 0;
    tmp |= (input[25].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[25].wrapping_sub(base)) >> (28 - 24);
    tmp |= (input[26].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[26].wrapping_sub(base)) >> (28 - 20);
    tmp |= (input[27].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[27].wrapping_sub(base)) >> (28 - 16);
    tmp |= (input[28].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[28].wrapping_sub(base)) >> (28 - 12);
    tmp |= (input[29].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[29].wrapping_sub(base)) >> (28 - 8);
    tmp |= (input[30].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[30].wrapping_sub(base)) >> (28 - 4);
    tmp |= (input[31].wrapping_sub(base)) << 4;
    write_partial(output, out_off, tmp, 4);
    112
}

pub fn unpack28_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 268435455);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 24)) << (28 - 24);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (28 - 20);
    output[2] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (28 - 16);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (28 - 12);
    output[4] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (28 - 8);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 8;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (28 - 4);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 4) & 268435455);
    in_off += 4;
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 0) & 268435455);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 24)) << (28 - 24);
    output[9] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (28 - 20);
    output[10] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (28 - 16);
    output[11] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (28 - 12);
    output[12] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (28 - 8);
    output[13] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 8;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (28 - 4);
    output[14] = base.wrapping_add(tmp);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 4) & 268435455);
    in_off += 4;
    output[16] = base.wrapping_add((read_u32(input, in_off) >> 0) & 268435455);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 24)) << (28 - 24);
    output[17] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (28 - 20);
    output[18] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (28 - 16);
    output[19] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (28 - 12);
    output[20] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (28 - 8);
    output[21] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 8;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (28 - 4);
    output[22] = base.wrapping_add(tmp);
    output[23] = base.wrapping_add((read_u32(input, in_off) >> 4) & 268435455);
    in_off += 4;
    output[24] = base.wrapping_add((read_u32(input, in_off) >> 0) & 268435455);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 24)) << (28 - 24);
    output[25] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (28 - 20);
    output[26] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (28 - 16);
    output[27] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (28 - 12);
    output[28] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (28 - 8);
    output[29] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 8;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (28 - 4);
    output[30] = base.wrapping_add(tmp);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 4) & 268435455);
    112
}

pub fn pack29_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (29 - 26);
    tmp |= (input[2].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (29 - 23);
    tmp |= (input[3].wrapping_sub(base)) << 23;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (29 - 20);
    tmp |= (input[4].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (29 - 17);
    tmp |= (input[5].wrapping_sub(base)) << 17;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (29 - 14);
    tmp |= (input[6].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (29 - 11);
    tmp |= (input[7].wrapping_sub(base)) << 11;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (29 - 8);
    tmp |= (input[8].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (29 - 5);
    tmp |= (input[9].wrapping_sub(base)) << 5;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (29 - 2);
    tmp |= (input[10].wrapping_sub(base)) << 2;
    tmp |= (input[11].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (29 - 28);
    tmp |= (input[12].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (29 - 25);
    tmp |= (input[13].wrapping_sub(base)) << 25;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (29 - 22);
    tmp |= (input[14].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (29 - 19);
    tmp |= (input[15].wrapping_sub(base)) << 19;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[15].wrapping_sub(base)) >> (29 - 16);
    tmp |= (input[16].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[16].wrapping_sub(base)) >> (29 - 13);
    tmp |= (input[17].wrapping_sub(base)) << 13;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[17].wrapping_sub(base)) >> (29 - 10);
    tmp |= (input[18].wrapping_sub(base)) << 10;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[18].wrapping_sub(base)) >> (29 - 7);
    tmp |= (input[19].wrapping_sub(base)) << 7;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[19].wrapping_sub(base)) >> (29 - 4);
    tmp |= (input[20].wrapping_sub(base)) << 4;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[20].wrapping_sub(base)) >> (29 - 1);
    tmp |= (input[21].wrapping_sub(base)) << 1;
    tmp |= (input[22].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[22].wrapping_sub(base)) >> (29 - 27);
    tmp |= (input[23].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[23].wrapping_sub(base)) >> (29 - 24);
    tmp |= (input[24].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[24].wrapping_sub(base)) >> (29 - 21);
    tmp |= (input[25].wrapping_sub(base)) << 21;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[25].wrapping_sub(base)) >> (29 - 18);
    tmp |= (input[26].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[26].wrapping_sub(base)) >> (29 - 15);
    tmp |= (input[27].wrapping_sub(base)) << 15;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[27].wrapping_sub(base)) >> (29 - 12);
    tmp |= (input[28].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[28].wrapping_sub(base)) >> (29 - 9);
    tmp |= (input[29].wrapping_sub(base)) << 9;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[29].wrapping_sub(base)) >> (29 - 6);
    tmp |= (input[30].wrapping_sub(base)) << 6;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[30].wrapping_sub(base)) >> (29 - 3);
    tmp |= (input[31].wrapping_sub(base)) << 3;
    write_partial(output, out_off, tmp, 4);
    116
}

pub fn unpack29_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 536870911);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 26)) << (29 - 26);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 23)) << (29 - 23);
    output[2] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 23;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (29 - 20);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 17)) << (29 - 17);
    output[4] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 17;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (29 - 14);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 14;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 11)) << (29 - 11);
    output[6] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 11;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (29 - 8);
    output[7] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 8;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 5)) << (29 - 5);
    output[8] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 5;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (29 - 2);
    output[9] = base.wrapping_add(tmp);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 2) & 536870911);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 28)) << (29 - 28);
    output[11] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 25)) << (29 - 25);
    output[12] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 25;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 22)) << (29 - 22);
    output[13] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 19)) << (29 - 19);
    output[14] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 19;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (29 - 16);
    output[15] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 13)) << (29 - 13);
    output[16] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 13;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (29 - 10);
    output[17] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 10;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 7)) << (29 - 7);
    output[18] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 7;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (29 - 4);
    output[19] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 4;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (29 - 1);
    output[20] = base.wrapping_add(tmp);
    output[21] = base.wrapping_add((read_u32(input, in_off) >> 1) & 536870911);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 27)) << (29 - 27);
    output[22] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 24)) << (29 - 24);
    output[23] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 21)) << (29 - 21);
    output[24] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 21;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 18)) << (29 - 18);
    output[25] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 15)) << (29 - 15);
    output[26] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 15;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (29 - 12);
    output[27] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 9)) << (29 - 9);
    output[28] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 9;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (29 - 6);
    output[29] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 6;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 3)) << (29 - 3);
    output[30] = base.wrapping_add(tmp);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 3) & 536870911);
    116
}

pub fn pack30_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (30 - 28);
    tmp |= (input[2].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (30 - 26);
    tmp |= (input[3].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (30 - 24);
    tmp |= (input[4].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (30 - 22);
    tmp |= (input[5].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (30 - 20);
    tmp |= (input[6].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (30 - 18);
    tmp |= (input[7].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (30 - 16);
    tmp |= (input[8].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (30 - 14);
    tmp |= (input[9].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (30 - 12);
    tmp |= (input[10].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (30 - 10);
    tmp |= (input[11].wrapping_sub(base)) << 10;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (30 - 8);
    tmp |= (input[12].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (30 - 6);
    tmp |= (input[13].wrapping_sub(base)) << 6;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (30 - 4);
    tmp |= (input[14].wrapping_sub(base)) << 4;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (30 - 2);
    tmp |= (input[15].wrapping_sub(base)) << 2;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[16].wrapping_sub(base)) << 0;
    tmp |= (input[17].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[17].wrapping_sub(base)) >> (30 - 28);
    tmp |= (input[18].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[18].wrapping_sub(base)) >> (30 - 26);
    tmp |= (input[19].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[19].wrapping_sub(base)) >> (30 - 24);
    tmp |= (input[20].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[20].wrapping_sub(base)) >> (30 - 22);
    tmp |= (input[21].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[21].wrapping_sub(base)) >> (30 - 20);
    tmp |= (input[22].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[22].wrapping_sub(base)) >> (30 - 18);
    tmp |= (input[23].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[23].wrapping_sub(base)) >> (30 - 16);
    tmp |= (input[24].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[24].wrapping_sub(base)) >> (30 - 14);
    tmp |= (input[25].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[25].wrapping_sub(base)) >> (30 - 12);
    tmp |= (input[26].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[26].wrapping_sub(base)) >> (30 - 10);
    tmp |= (input[27].wrapping_sub(base)) << 10;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[27].wrapping_sub(base)) >> (30 - 8);
    tmp |= (input[28].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[28].wrapping_sub(base)) >> (30 - 6);
    tmp |= (input[29].wrapping_sub(base)) << 6;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[29].wrapping_sub(base)) >> (30 - 4);
    tmp |= (input[30].wrapping_sub(base)) << 4;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[30].wrapping_sub(base)) >> (30 - 2);
    tmp |= (input[31].wrapping_sub(base)) << 2;
    write_partial(output, out_off, tmp, 4);
    120
}

pub fn unpack30_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 1073741823);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 28)) << (30 - 28);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 26)) << (30 - 26);
    output[2] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 24)) << (30 - 24);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 22)) << (30 - 22);
    output[4] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (30 - 20);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 18)) << (30 - 18);
    output[6] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (30 - 16);
    output[7] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (30 - 14);
    output[8] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 14;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (30 - 12);
    output[9] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (30 - 10);
    output[10] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 10;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (30 - 8);
    output[11] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 8;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (30 - 6);
    output[12] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 6;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (30 - 4);
    output[13] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 4;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (30 - 2);
    output[14] = base.wrapping_add(tmp);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 2) & 1073741823);
    in_off += 4;
    output[16] = base.wrapping_add((read_u32(input, in_off) >> 0) & 1073741823);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 28)) << (30 - 28);
    output[17] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 26)) << (30 - 26);
    output[18] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 24)) << (30 - 24);
    output[19] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 22)) << (30 - 22);
    output[20] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (30 - 20);
    output[21] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 18)) << (30 - 18);
    output[22] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (30 - 16);
    output[23] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (30 - 14);
    output[24] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 14;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (30 - 12);
    output[25] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (30 - 10);
    output[26] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 10;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (30 - 8);
    output[27] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 8;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (30 - 6);
    output[28] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 6;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (30 - 4);
    output[29] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 4;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (30 - 2);
    output[30] = base.wrapping_add(tmp);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 2) & 1073741823);
    120
}

pub fn pack31_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (31 - 30);
    tmp |= (input[2].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (31 - 29);
    tmp |= (input[3].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (31 - 28);
    tmp |= (input[4].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (31 - 27);
    tmp |= (input[5].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (31 - 26);
    tmp |= (input[6].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (31 - 25);
    tmp |= (input[7].wrapping_sub(base)) << 25;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (31 - 24);
    tmp |= (input[8].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (31 - 23);
    tmp |= (input[9].wrapping_sub(base)) << 23;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (31 - 22);
    tmp |= (input[10].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (31 - 21);
    tmp |= (input[11].wrapping_sub(base)) << 21;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (31 - 20);
    tmp |= (input[12].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (31 - 19);
    tmp |= (input[13].wrapping_sub(base)) << 19;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (31 - 18);
    tmp |= (input[14].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (31 - 17);
    tmp |= (input[15].wrapping_sub(base)) << 17;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[15].wrapping_sub(base)) >> (31 - 16);
    tmp |= (input[16].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[16].wrapping_sub(base)) >> (31 - 15);
    tmp |= (input[17].wrapping_sub(base)) << 15;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[17].wrapping_sub(base)) >> (31 - 14);
    tmp |= (input[18].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[18].wrapping_sub(base)) >> (31 - 13);
    tmp |= (input[19].wrapping_sub(base)) << 13;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[19].wrapping_sub(base)) >> (31 - 12);
    tmp |= (input[20].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[20].wrapping_sub(base)) >> (31 - 11);
    tmp |= (input[21].wrapping_sub(base)) << 11;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[21].wrapping_sub(base)) >> (31 - 10);
    tmp |= (input[22].wrapping_sub(base)) << 10;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[22].wrapping_sub(base)) >> (31 - 9);
    tmp |= (input[23].wrapping_sub(base)) << 9;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[23].wrapping_sub(base)) >> (31 - 8);
    tmp |= (input[24].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[24].wrapping_sub(base)) >> (31 - 7);
    tmp |= (input[25].wrapping_sub(base)) << 7;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[25].wrapping_sub(base)) >> (31 - 6);
    tmp |= (input[26].wrapping_sub(base)) << 6;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[26].wrapping_sub(base)) >> (31 - 5);
    tmp |= (input[27].wrapping_sub(base)) << 5;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[27].wrapping_sub(base)) >> (31 - 4);
    tmp |= (input[28].wrapping_sub(base)) << 4;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[28].wrapping_sub(base)) >> (31 - 3);
    tmp |= (input[29].wrapping_sub(base)) << 3;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[29].wrapping_sub(base)) >> (31 - 2);
    tmp |= (input[30].wrapping_sub(base)) << 2;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[30].wrapping_sub(base)) >> (31 - 1);
    tmp |= (input[31].wrapping_sub(base)) << 1;
    write_partial(output, out_off, tmp, 4);
    124
}

pub fn unpack31_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 2147483647);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 30)) << (31 - 30);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 29)) << (31 - 29);
    output[2] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 28)) << (31 - 28);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 27)) << (31 - 27);
    output[4] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 26)) << (31 - 26);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 25)) << (31 - 25);
    output[6] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 25;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 24)) << (31 - 24);
    output[7] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 23)) << (31 - 23);
    output[8] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 23;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 22)) << (31 - 22);
    output[9] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 21)) << (31 - 21);
    output[10] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 21;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (31 - 20);
    output[11] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 19)) << (31 - 19);
    output[12] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 19;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 18)) << (31 - 18);
    output[13] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 17)) << (31 - 17);
    output[14] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 17;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (31 - 16);
    output[15] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 15)) << (31 - 15);
    output[16] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 15;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (31 - 14);
    output[17] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 14;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 13)) << (31 - 13);
    output[18] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 13;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (31 - 12);
    output[19] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 11)) << (31 - 11);
    output[20] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 11;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (31 - 10);
    output[21] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 10;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 9)) << (31 - 9);
    output[22] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 9;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (31 - 8);
    output[23] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 8;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 7)) << (31 - 7);
    output[24] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 7;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (31 - 6);
    output[25] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 6;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 5)) << (31 - 5);
    output[26] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 5;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (31 - 4);
    output[27] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 4;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 3)) << (31 - 3);
    output[28] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 3;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (31 - 2);
    output[29] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 2;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (31 - 1);
    output[30] = base.wrapping_add(tmp);
    output[31] = base.wrapping_add((read_u32(input, in_off) >> 1) & 2147483647);
    124
}

pub fn pack32_32(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    for i in 0..32 {
        write_u32(output, i * 4, input[i].wrapping_sub(base));
    }
    128
}

pub fn unpack32_32(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    for i in 0..32 {
        output[i] = base.wrapping_add(read_u32(input, i * 4));
    }
    128
}

pub fn pack1_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 1;
    tmp |= (input[2].wrapping_sub(base)) << 2;
    tmp |= (input[3].wrapping_sub(base)) << 3;
    tmp |= (input[4].wrapping_sub(base)) << 4;
    tmp |= (input[5].wrapping_sub(base)) << 5;
    tmp |= (input[6].wrapping_sub(base)) << 6;
    tmp |= (input[7].wrapping_sub(base)) << 7;
    tmp |= (input[8].wrapping_sub(base)) << 8;
    tmp |= (input[9].wrapping_sub(base)) << 9;
    tmp |= (input[10].wrapping_sub(base)) << 10;
    tmp |= (input[11].wrapping_sub(base)) << 11;
    tmp |= (input[12].wrapping_sub(base)) << 12;
    tmp |= (input[13].wrapping_sub(base)) << 13;
    tmp |= (input[14].wrapping_sub(base)) << 14;
    tmp |= (input[15].wrapping_sub(base)) << 15;
    write_partial(output, out_off, tmp, 2);
    2
}

pub fn unpack1_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 1);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 1) & 1);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 2) & 1);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 3) & 1);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 4) & 1);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 5) & 1);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 6) & 1);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 7) & 1);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 8) & 1);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 9) & 1);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 10) & 1);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 11) & 1);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 12) & 1);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 13) & 1);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 14) & 1);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 15) & 1);
    2
}

pub fn pack2_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 2;
    tmp |= (input[2].wrapping_sub(base)) << 4;
    tmp |= (input[3].wrapping_sub(base)) << 6;
    tmp |= (input[4].wrapping_sub(base)) << 8;
    tmp |= (input[5].wrapping_sub(base)) << 10;
    tmp |= (input[6].wrapping_sub(base)) << 12;
    tmp |= (input[7].wrapping_sub(base)) << 14;
    tmp |= (input[8].wrapping_sub(base)) << 16;
    tmp |= (input[9].wrapping_sub(base)) << 18;
    tmp |= (input[10].wrapping_sub(base)) << 20;
    tmp |= (input[11].wrapping_sub(base)) << 22;
    tmp |= (input[12].wrapping_sub(base)) << 24;
    tmp |= (input[13].wrapping_sub(base)) << 26;
    tmp |= (input[14].wrapping_sub(base)) << 28;
    tmp |= (input[15].wrapping_sub(base)) << 30;
    write_partial(output, out_off, tmp, 4);
    4
}

pub fn unpack2_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 3);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 2) & 3);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 4) & 3);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 6) & 3);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 8) & 3);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 10) & 3);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 12) & 3);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 14) & 3);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 16) & 3);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 18) & 3);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 20) & 3);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 22) & 3);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 24) & 3);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 26) & 3);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 28) & 3);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 30) & 3);
    4
}

pub fn pack3_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 3;
    tmp |= (input[2].wrapping_sub(base)) << 6;
    tmp |= (input[3].wrapping_sub(base)) << 9;
    tmp |= (input[4].wrapping_sub(base)) << 12;
    tmp |= (input[5].wrapping_sub(base)) << 15;
    tmp |= (input[6].wrapping_sub(base)) << 18;
    tmp |= (input[7].wrapping_sub(base)) << 21;
    tmp |= (input[8].wrapping_sub(base)) << 24;
    tmp |= (input[9].wrapping_sub(base)) << 27;
    tmp |= (input[10].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (3 - 1);
    tmp |= (input[11].wrapping_sub(base)) << 1;
    tmp |= (input[12].wrapping_sub(base)) << 4;
    tmp |= (input[13].wrapping_sub(base)) << 7;
    tmp |= (input[14].wrapping_sub(base)) << 10;
    tmp |= (input[15].wrapping_sub(base)) << 13;
    write_partial(output, out_off, tmp, 2);
    6
}

pub fn unpack3_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 7);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 3) & 7);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 6) & 7);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 9) & 7);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 12) & 7);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 15) & 7);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 18) & 7);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 21) & 7);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 24) & 7);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 27) & 7);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (3 - 1);
    output[10] = base.wrapping_add(tmp);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 1) & 7);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 4) & 7);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 7) & 7);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 10) & 7);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 13) & 7);
    6
}

pub fn pack4_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 4;
    tmp |= (input[2].wrapping_sub(base)) << 8;
    tmp |= (input[3].wrapping_sub(base)) << 12;
    tmp |= (input[4].wrapping_sub(base)) << 16;
    tmp |= (input[5].wrapping_sub(base)) << 20;
    tmp |= (input[6].wrapping_sub(base)) << 24;
    tmp |= (input[7].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) << 0;
    tmp |= (input[9].wrapping_sub(base)) << 4;
    tmp |= (input[10].wrapping_sub(base)) << 8;
    tmp |= (input[11].wrapping_sub(base)) << 12;
    tmp |= (input[12].wrapping_sub(base)) << 16;
    tmp |= (input[13].wrapping_sub(base)) << 20;
    tmp |= (input[14].wrapping_sub(base)) << 24;
    tmp |= (input[15].wrapping_sub(base)) << 28;
    write_partial(output, out_off, tmp, 4);
    8
}

pub fn unpack4_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 15);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 4) & 15);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 8) & 15);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 12) & 15);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 16) & 15);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 20) & 15);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 24) & 15);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 28) & 15);
    in_off += 4;
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 0) & 15);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 4) & 15);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 8) & 15);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 12) & 15);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 16) & 15);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 20) & 15);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 24) & 15);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 28) & 15);
    8
}

pub fn pack5_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 5;
    tmp |= (input[2].wrapping_sub(base)) << 10;
    tmp |= (input[3].wrapping_sub(base)) << 15;
    tmp |= (input[4].wrapping_sub(base)) << 20;
    tmp |= (input[5].wrapping_sub(base)) << 25;
    tmp |= (input[6].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (5 - 3);
    tmp |= (input[7].wrapping_sub(base)) << 3;
    tmp |= (input[8].wrapping_sub(base)) << 8;
    tmp |= (input[9].wrapping_sub(base)) << 13;
    tmp |= (input[10].wrapping_sub(base)) << 18;
    tmp |= (input[11].wrapping_sub(base)) << 23;
    tmp |= (input[12].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (5 - 1);
    tmp |= (input[13].wrapping_sub(base)) << 1;
    tmp |= (input[14].wrapping_sub(base)) << 6;
    tmp |= (input[15].wrapping_sub(base)) << 11;
    write_partial(output, out_off, tmp, 2);
    10
}

pub fn unpack5_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 31);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 5) & 31);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 10) & 31);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 15) & 31);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 20) & 31);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 25) & 31);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 3)) << (5 - 3);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 3) & 31);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 8) & 31);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 13) & 31);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 18) & 31);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 23) & 31);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (5 - 1);
    output[12] = base.wrapping_add(tmp);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 1) & 31);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 6) & 31);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 11) & 31);
    10
}

pub fn pack6_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 6;
    tmp |= (input[2].wrapping_sub(base)) << 12;
    tmp |= (input[3].wrapping_sub(base)) << 18;
    tmp |= (input[4].wrapping_sub(base)) << 24;
    tmp |= (input[5].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (6 - 4);
    tmp |= (input[6].wrapping_sub(base)) << 4;
    tmp |= (input[7].wrapping_sub(base)) << 10;
    tmp |= (input[8].wrapping_sub(base)) << 16;
    tmp |= (input[9].wrapping_sub(base)) << 22;
    tmp |= (input[10].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (6 - 2);
    tmp |= (input[11].wrapping_sub(base)) << 2;
    tmp |= (input[12].wrapping_sub(base)) << 8;
    tmp |= (input[13].wrapping_sub(base)) << 14;
    tmp |= (input[14].wrapping_sub(base)) << 20;
    tmp |= (input[15].wrapping_sub(base)) << 26;
    write_partial(output, out_off, tmp, 4);
    12
}

pub fn unpack6_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 63);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 6) & 63);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 12) & 63);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 18) & 63);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 24) & 63);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (6 - 4);
    output[5] = base.wrapping_add(tmp);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 4) & 63);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 10) & 63);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 16) & 63);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 22) & 63);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (6 - 2);
    output[10] = base.wrapping_add(tmp);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 2) & 63);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 8) & 63);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 14) & 63);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 20) & 63);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 26) & 63);
    12
}

pub fn pack7_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 7;
    tmp |= (input[2].wrapping_sub(base)) << 14;
    tmp |= (input[3].wrapping_sub(base)) << 21;
    tmp |= (input[4].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (7 - 3);
    tmp |= (input[5].wrapping_sub(base)) << 3;
    tmp |= (input[6].wrapping_sub(base)) << 10;
    tmp |= (input[7].wrapping_sub(base)) << 17;
    tmp |= (input[8].wrapping_sub(base)) << 24;
    tmp |= (input[9].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (7 - 6);
    tmp |= (input[10].wrapping_sub(base)) << 6;
    tmp |= (input[11].wrapping_sub(base)) << 13;
    tmp |= (input[12].wrapping_sub(base)) << 20;
    tmp |= (input[13].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (7 - 2);
    tmp |= (input[14].wrapping_sub(base)) << 2;
    tmp |= (input[15].wrapping_sub(base)) << 9;
    write_partial(output, out_off, tmp, 2);
    14
}

pub fn unpack7_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 127);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 7) & 127);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 14) & 127);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 21) & 127);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 3)) << (7 - 3);
    output[4] = base.wrapping_add(tmp);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 3) & 127);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 10) & 127);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 17) & 127);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 24) & 127);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (7 - 6);
    output[9] = base.wrapping_add(tmp);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 6) & 127);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 13) & 127);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 20) & 127);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (7 - 2);
    output[13] = base.wrapping_add(tmp);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 2) & 127);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 9) & 127);
    14
}

pub fn pack8_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 8;
    tmp |= (input[2].wrapping_sub(base)) << 16;
    tmp |= (input[3].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) << 0;
    tmp |= (input[5].wrapping_sub(base)) << 8;
    tmp |= (input[6].wrapping_sub(base)) << 16;
    tmp |= (input[7].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) << 0;
    tmp |= (input[9].wrapping_sub(base)) << 8;
    tmp |= (input[10].wrapping_sub(base)) << 16;
    tmp |= (input[11].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) << 0;
    tmp |= (input[13].wrapping_sub(base)) << 8;
    tmp |= (input[14].wrapping_sub(base)) << 16;
    tmp |= (input[15].wrapping_sub(base)) << 24;
    write_partial(output, out_off, tmp, 4);
    16
}

pub fn unpack8_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 255);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 8) & 255);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 16) & 255);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 24) & 255);
    in_off += 4;
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 0) & 255);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 8) & 255);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 16) & 255);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 24) & 255);
    in_off += 4;
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 0) & 255);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 8) & 255);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 16) & 255);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 24) & 255);
    in_off += 4;
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 0) & 255);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 8) & 255);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 16) & 255);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 24) & 255);
    16
}

pub fn pack9_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 9;
    tmp |= (input[2].wrapping_sub(base)) << 18;
    tmp |= (input[3].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (9 - 4);
    tmp |= (input[4].wrapping_sub(base)) << 4;
    tmp |= (input[5].wrapping_sub(base)) << 13;
    tmp |= (input[6].wrapping_sub(base)) << 22;
    tmp |= (input[7].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (9 - 8);
    tmp |= (input[8].wrapping_sub(base)) << 8;
    tmp |= (input[9].wrapping_sub(base)) << 17;
    tmp |= (input[10].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (9 - 3);
    tmp |= (input[11].wrapping_sub(base)) << 3;
    tmp |= (input[12].wrapping_sub(base)) << 12;
    tmp |= (input[13].wrapping_sub(base)) << 21;
    tmp |= (input[14].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (9 - 7);
    tmp |= (input[15].wrapping_sub(base)) << 7;
    write_partial(output, out_off, tmp, 2);
    18
}

pub fn unpack9_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 511);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 9) & 511);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 18) & 511);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (9 - 4);
    output[3] = base.wrapping_add(tmp);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 4) & 511);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 13) & 511);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 22) & 511);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (9 - 8);
    output[7] = base.wrapping_add(tmp);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 8) & 511);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 17) & 511);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 3)) << (9 - 3);
    output[10] = base.wrapping_add(tmp);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 3) & 511);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 12) & 511);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 21) & 511);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 7)) << (9 - 7);
    output[14] = base.wrapping_add(tmp);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 7) & 511);
    18
}

pub fn pack10_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 10;
    tmp |= (input[2].wrapping_sub(base)) << 20;
    tmp |= (input[3].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (10 - 8);
    tmp |= (input[4].wrapping_sub(base)) << 8;
    tmp |= (input[5].wrapping_sub(base)) << 18;
    tmp |= (input[6].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (10 - 6);
    tmp |= (input[7].wrapping_sub(base)) << 6;
    tmp |= (input[8].wrapping_sub(base)) << 16;
    tmp |= (input[9].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (10 - 4);
    tmp |= (input[10].wrapping_sub(base)) << 4;
    tmp |= (input[11].wrapping_sub(base)) << 14;
    tmp |= (input[12].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (10 - 2);
    tmp |= (input[13].wrapping_sub(base)) << 2;
    tmp |= (input[14].wrapping_sub(base)) << 12;
    tmp |= (input[15].wrapping_sub(base)) << 22;
    write_partial(output, out_off, tmp, 4);
    20
}

pub fn unpack10_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 1023);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 10) & 1023);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 20) & 1023);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (10 - 8);
    output[3] = base.wrapping_add(tmp);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 8) & 1023);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 18) & 1023);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (10 - 6);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 6) & 1023);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 16) & 1023);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (10 - 4);
    output[9] = base.wrapping_add(tmp);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 4) & 1023);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 14) & 1023);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (10 - 2);
    output[12] = base.wrapping_add(tmp);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 2) & 1023);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 12) & 1023);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 22) & 1023);
    20
}

pub fn pack11_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 11;
    tmp |= (input[2].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (11 - 1);
    tmp |= (input[3].wrapping_sub(base)) << 1;
    tmp |= (input[4].wrapping_sub(base)) << 12;
    tmp |= (input[5].wrapping_sub(base)) << 23;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (11 - 2);
    tmp |= (input[6].wrapping_sub(base)) << 2;
    tmp |= (input[7].wrapping_sub(base)) << 13;
    tmp |= (input[8].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (11 - 3);
    tmp |= (input[9].wrapping_sub(base)) << 3;
    tmp |= (input[10].wrapping_sub(base)) << 14;
    tmp |= (input[11].wrapping_sub(base)) << 25;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (11 - 4);
    tmp |= (input[12].wrapping_sub(base)) << 4;
    tmp |= (input[13].wrapping_sub(base)) << 15;
    tmp |= (input[14].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (11 - 5);
    tmp |= (input[15].wrapping_sub(base)) << 5;
    write_partial(output, out_off, tmp, 2);
    22
}

pub fn unpack11_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 2047);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 11) & 2047);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (11 - 1);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 1) & 2047);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 12) & 2047);
    tmp = read_u32(input, in_off) >> 23;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (11 - 2);
    output[5] = base.wrapping_add(tmp);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 2) & 2047);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 13) & 2047);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 3)) << (11 - 3);
    output[8] = base.wrapping_add(tmp);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 3) & 2047);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 14) & 2047);
    tmp = read_u32(input, in_off) >> 25;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (11 - 4);
    output[11] = base.wrapping_add(tmp);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 4) & 2047);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 15) & 2047);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 5)) << (11 - 5);
    output[14] = base.wrapping_add(tmp);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 5) & 2047);
    22
}

pub fn pack12_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 12;
    tmp |= (input[2].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (12 - 4);
    tmp |= (input[3].wrapping_sub(base)) << 4;
    tmp |= (input[4].wrapping_sub(base)) << 16;
    tmp |= (input[5].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (12 - 8);
    tmp |= (input[6].wrapping_sub(base)) << 8;
    tmp |= (input[7].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) << 0;
    tmp |= (input[9].wrapping_sub(base)) << 12;
    tmp |= (input[10].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (12 - 4);
    tmp |= (input[11].wrapping_sub(base)) << 4;
    tmp |= (input[12].wrapping_sub(base)) << 16;
    tmp |= (input[13].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (12 - 8);
    tmp |= (input[14].wrapping_sub(base)) << 8;
    tmp |= (input[15].wrapping_sub(base)) << 20;
    write_partial(output, out_off, tmp, 4);
    24
}

pub fn unpack12_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 4095);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 12) & 4095);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (12 - 4);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 4) & 4095);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 16) & 4095);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (12 - 8);
    output[5] = base.wrapping_add(tmp);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 8) & 4095);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 20) & 4095);
    in_off += 4;
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 0) & 4095);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 12) & 4095);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (12 - 4);
    output[10] = base.wrapping_add(tmp);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 4) & 4095);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 16) & 4095);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (12 - 8);
    output[13] = base.wrapping_add(tmp);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 8) & 4095);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 20) & 4095);
    24
}

pub fn pack13_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 13;
    tmp |= (input[2].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (13 - 7);
    tmp |= (input[3].wrapping_sub(base)) << 7;
    tmp |= (input[4].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (13 - 1);
    tmp |= (input[5].wrapping_sub(base)) << 1;
    tmp |= (input[6].wrapping_sub(base)) << 14;
    tmp |= (input[7].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (13 - 8);
    tmp |= (input[8].wrapping_sub(base)) << 8;
    tmp |= (input[9].wrapping_sub(base)) << 21;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (13 - 2);
    tmp |= (input[10].wrapping_sub(base)) << 2;
    tmp |= (input[11].wrapping_sub(base)) << 15;
    tmp |= (input[12].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (13 - 9);
    tmp |= (input[13].wrapping_sub(base)) << 9;
    tmp |= (input[14].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (13 - 3);
    tmp |= (input[15].wrapping_sub(base)) << 3;
    write_partial(output, out_off, tmp, 2);
    26
}

pub fn unpack13_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 8191);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 13) & 8191);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 7)) << (13 - 7);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 7) & 8191);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (13 - 1);
    output[4] = base.wrapping_add(tmp);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 1) & 8191);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 14) & 8191);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (13 - 8);
    output[7] = base.wrapping_add(tmp);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 8) & 8191);
    tmp = read_u32(input, in_off) >> 21;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (13 - 2);
    output[9] = base.wrapping_add(tmp);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 2) & 8191);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 15) & 8191);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 9)) << (13 - 9);
    output[12] = base.wrapping_add(tmp);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 9) & 8191);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 3)) << (13 - 3);
    output[14] = base.wrapping_add(tmp);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 3) & 8191);
    26
}

pub fn pack14_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 14;
    tmp |= (input[2].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (14 - 10);
    tmp |= (input[3].wrapping_sub(base)) << 10;
    tmp |= (input[4].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (14 - 6);
    tmp |= (input[5].wrapping_sub(base)) << 6;
    tmp |= (input[6].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (14 - 2);
    tmp |= (input[7].wrapping_sub(base)) << 2;
    tmp |= (input[8].wrapping_sub(base)) << 16;
    tmp |= (input[9].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (14 - 12);
    tmp |= (input[10].wrapping_sub(base)) << 12;
    tmp |= (input[11].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (14 - 8);
    tmp |= (input[12].wrapping_sub(base)) << 8;
    tmp |= (input[13].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (14 - 4);
    tmp |= (input[14].wrapping_sub(base)) << 4;
    tmp |= (input[15].wrapping_sub(base)) << 18;
    write_partial(output, out_off, tmp, 4);
    28
}

pub fn unpack14_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 16383);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 14) & 16383);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (14 - 10);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 10) & 16383);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (14 - 6);
    output[4] = base.wrapping_add(tmp);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 6) & 16383);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (14 - 2);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 2) & 16383);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 16) & 16383);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (14 - 12);
    output[9] = base.wrapping_add(tmp);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 12) & 16383);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (14 - 8);
    output[11] = base.wrapping_add(tmp);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 8) & 16383);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (14 - 4);
    output[13] = base.wrapping_add(tmp);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 4) & 16383);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 18) & 16383);
    28
}

pub fn pack15_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 15;
    tmp |= (input[2].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (15 - 13);
    tmp |= (input[3].wrapping_sub(base)) << 13;
    tmp |= (input[4].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (15 - 11);
    tmp |= (input[5].wrapping_sub(base)) << 11;
    tmp |= (input[6].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (15 - 9);
    tmp |= (input[7].wrapping_sub(base)) << 9;
    tmp |= (input[8].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (15 - 7);
    tmp |= (input[9].wrapping_sub(base)) << 7;
    tmp |= (input[10].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (15 - 5);
    tmp |= (input[11].wrapping_sub(base)) << 5;
    tmp |= (input[12].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (15 - 3);
    tmp |= (input[13].wrapping_sub(base)) << 3;
    tmp |= (input[14].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (15 - 1);
    tmp |= (input[15].wrapping_sub(base)) << 1;
    write_partial(output, out_off, tmp, 2);
    30
}

pub fn unpack15_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 32767);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 15) & 32767);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 13)) << (15 - 13);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 13) & 32767);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 11)) << (15 - 11);
    output[4] = base.wrapping_add(tmp);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 11) & 32767);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 9)) << (15 - 9);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 9) & 32767);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 7)) << (15 - 7);
    output[8] = base.wrapping_add(tmp);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 7) & 32767);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 5)) << (15 - 5);
    output[10] = base.wrapping_add(tmp);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 5) & 32767);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 3)) << (15 - 3);
    output[12] = base.wrapping_add(tmp);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 3) & 32767);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (15 - 1);
    output[14] = base.wrapping_add(tmp);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 1) & 32767);
    30
}

pub fn pack16_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) << 0;
    tmp |= (input[3].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) << 0;
    tmp |= (input[5].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) << 0;
    tmp |= (input[7].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) << 0;
    tmp |= (input[9].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) << 0;
    tmp |= (input[11].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) << 0;
    tmp |= (input[13].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) << 0;
    tmp |= (input[15].wrapping_sub(base)) << 16;
    write_partial(output, out_off, tmp, 4);
    32
}

pub fn unpack16_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    32
}

pub fn pack17_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 17;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (17 - 2);
    tmp |= (input[2].wrapping_sub(base)) << 2;
    tmp |= (input[3].wrapping_sub(base)) << 19;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (17 - 4);
    tmp |= (input[4].wrapping_sub(base)) << 4;
    tmp |= (input[5].wrapping_sub(base)) << 21;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (17 - 6);
    tmp |= (input[6].wrapping_sub(base)) << 6;
    tmp |= (input[7].wrapping_sub(base)) << 23;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (17 - 8);
    tmp |= (input[8].wrapping_sub(base)) << 8;
    tmp |= (input[9].wrapping_sub(base)) << 25;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (17 - 10);
    tmp |= (input[10].wrapping_sub(base)) << 10;
    tmp |= (input[11].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (17 - 12);
    tmp |= (input[12].wrapping_sub(base)) << 12;
    tmp |= (input[13].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (17 - 14);
    tmp |= (input[14].wrapping_sub(base)) << 14;
    tmp |= (input[15].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[15].wrapping_sub(base)) >> (17 - 16);
    write_partial(output, out_off, tmp, 2);
    34
}

pub fn unpack17_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 131071);
    tmp = read_u32(input, in_off) >> 17;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (17 - 2);
    output[1] = base.wrapping_add(tmp);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 2) & 131071);
    tmp = read_u32(input, in_off) >> 19;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (17 - 4);
    output[3] = base.wrapping_add(tmp);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 4) & 131071);
    tmp = read_u32(input, in_off) >> 21;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (17 - 6);
    output[5] = base.wrapping_add(tmp);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 6) & 131071);
    tmp = read_u32(input, in_off) >> 23;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (17 - 8);
    output[7] = base.wrapping_add(tmp);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 8) & 131071);
    tmp = read_u32(input, in_off) >> 25;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (17 - 10);
    output[9] = base.wrapping_add(tmp);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 10) & 131071);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (17 - 12);
    output[11] = base.wrapping_add(tmp);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 12) & 131071);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (17 - 14);
    output[13] = base.wrapping_add(tmp);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 14) & 131071);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (17 - 16);
    output[15] = base.wrapping_add(tmp);
    34
}

pub fn pack18_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (18 - 4);
    tmp |= (input[2].wrapping_sub(base)) << 4;
    tmp |= (input[3].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (18 - 8);
    tmp |= (input[4].wrapping_sub(base)) << 8;
    tmp |= (input[5].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (18 - 12);
    tmp |= (input[6].wrapping_sub(base)) << 12;
    tmp |= (input[7].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (18 - 16);
    tmp |= (input[8].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (18 - 2);
    tmp |= (input[9].wrapping_sub(base)) << 2;
    tmp |= (input[10].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (18 - 6);
    tmp |= (input[11].wrapping_sub(base)) << 6;
    tmp |= (input[12].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (18 - 10);
    tmp |= (input[13].wrapping_sub(base)) << 10;
    tmp |= (input[14].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (18 - 14);
    tmp |= (input[15].wrapping_sub(base)) << 14;
    write_partial(output, out_off, tmp, 4);
    36
}

pub fn unpack18_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 262143);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (18 - 4);
    output[1] = base.wrapping_add(tmp);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 4) & 262143);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (18 - 8);
    output[3] = base.wrapping_add(tmp);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 8) & 262143);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (18 - 12);
    output[5] = base.wrapping_add(tmp);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 12) & 262143);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (18 - 16);
    output[7] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (18 - 2);
    output[8] = base.wrapping_add(tmp);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 2) & 262143);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (18 - 6);
    output[10] = base.wrapping_add(tmp);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 6) & 262143);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (18 - 10);
    output[12] = base.wrapping_add(tmp);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 10) & 262143);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (18 - 14);
    output[14] = base.wrapping_add(tmp);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 14) & 262143);
    36
}

pub fn pack19_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 19;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (19 - 6);
    tmp |= (input[2].wrapping_sub(base)) << 6;
    tmp |= (input[3].wrapping_sub(base)) << 25;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (19 - 12);
    tmp |= (input[4].wrapping_sub(base)) << 12;
    tmp |= (input[5].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (19 - 18);
    tmp |= (input[6].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (19 - 5);
    tmp |= (input[7].wrapping_sub(base)) << 5;
    tmp |= (input[8].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (19 - 11);
    tmp |= (input[9].wrapping_sub(base)) << 11;
    tmp |= (input[10].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (19 - 17);
    tmp |= (input[11].wrapping_sub(base)) << 17;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (19 - 4);
    tmp |= (input[12].wrapping_sub(base)) << 4;
    tmp |= (input[13].wrapping_sub(base)) << 23;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (19 - 10);
    tmp |= (input[14].wrapping_sub(base)) << 10;
    tmp |= (input[15].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[15].wrapping_sub(base)) >> (19 - 16);
    write_partial(output, out_off, tmp, 2);
    38
}

pub fn unpack19_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 524287);
    tmp = read_u32(input, in_off) >> 19;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (19 - 6);
    output[1] = base.wrapping_add(tmp);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 6) & 524287);
    tmp = read_u32(input, in_off) >> 25;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (19 - 12);
    output[3] = base.wrapping_add(tmp);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 12) & 524287);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 18)) << (19 - 18);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 5)) << (19 - 5);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 5) & 524287);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 11)) << (19 - 11);
    output[8] = base.wrapping_add(tmp);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 11) & 524287);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 17)) << (19 - 17);
    output[10] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 17;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (19 - 4);
    output[11] = base.wrapping_add(tmp);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 4) & 524287);
    tmp = read_u32(input, in_off) >> 23;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (19 - 10);
    output[13] = base.wrapping_add(tmp);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 10) & 524287);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (19 - 16);
    output[15] = base.wrapping_add(tmp);
    38
}

pub fn pack20_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (20 - 8);
    tmp |= (input[2].wrapping_sub(base)) << 8;
    tmp |= (input[3].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (20 - 16);
    tmp |= (input[4].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (20 - 4);
    tmp |= (input[5].wrapping_sub(base)) << 4;
    tmp |= (input[6].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (20 - 12);
    tmp |= (input[7].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) << 0;
    tmp |= (input[9].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (20 - 8);
    tmp |= (input[10].wrapping_sub(base)) << 8;
    tmp |= (input[11].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (20 - 16);
    tmp |= (input[12].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (20 - 4);
    tmp |= (input[13].wrapping_sub(base)) << 4;
    tmp |= (input[14].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (20 - 12);
    tmp |= (input[15].wrapping_sub(base)) << 12;
    write_partial(output, out_off, tmp, 4);
    40
}

pub fn unpack20_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 1048575);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (20 - 8);
    output[1] = base.wrapping_add(tmp);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 8) & 1048575);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (20 - 16);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (20 - 4);
    output[4] = base.wrapping_add(tmp);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 4) & 1048575);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (20 - 12);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 12) & 1048575);
    in_off += 4;
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 0) & 1048575);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (20 - 8);
    output[9] = base.wrapping_add(tmp);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 8) & 1048575);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (20 - 16);
    output[11] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (20 - 4);
    output[12] = base.wrapping_add(tmp);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 4) & 1048575);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (20 - 12);
    output[14] = base.wrapping_add(tmp);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 12) & 1048575);
    40
}

pub fn pack21_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 21;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (21 - 10);
    tmp |= (input[2].wrapping_sub(base)) << 10;
    tmp |= (input[3].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (21 - 20);
    tmp |= (input[4].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (21 - 9);
    tmp |= (input[5].wrapping_sub(base)) << 9;
    tmp |= (input[6].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (21 - 19);
    tmp |= (input[7].wrapping_sub(base)) << 19;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (21 - 8);
    tmp |= (input[8].wrapping_sub(base)) << 8;
    tmp |= (input[9].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (21 - 18);
    tmp |= (input[10].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (21 - 7);
    tmp |= (input[11].wrapping_sub(base)) << 7;
    tmp |= (input[12].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (21 - 17);
    tmp |= (input[13].wrapping_sub(base)) << 17;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (21 - 6);
    tmp |= (input[14].wrapping_sub(base)) << 6;
    tmp |= (input[15].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[15].wrapping_sub(base)) >> (21 - 16);
    write_partial(output, out_off, tmp, 2);
    42
}

pub fn unpack21_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 2097151);
    tmp = read_u32(input, in_off) >> 21;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (21 - 10);
    output[1] = base.wrapping_add(tmp);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 10) & 2097151);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (21 - 20);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 9)) << (21 - 9);
    output[4] = base.wrapping_add(tmp);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 9) & 2097151);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 19)) << (21 - 19);
    output[6] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 19;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (21 - 8);
    output[7] = base.wrapping_add(tmp);
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 8) & 2097151);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 18)) << (21 - 18);
    output[9] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 7)) << (21 - 7);
    output[10] = base.wrapping_add(tmp);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 7) & 2097151);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 17)) << (21 - 17);
    output[12] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 17;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (21 - 6);
    output[13] = base.wrapping_add(tmp);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 6) & 2097151);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (21 - 16);
    output[15] = base.wrapping_add(tmp);
    42
}

pub fn pack22_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (22 - 12);
    tmp |= (input[2].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (22 - 2);
    tmp |= (input[3].wrapping_sub(base)) << 2;
    tmp |= (input[4].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (22 - 14);
    tmp |= (input[5].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (22 - 4);
    tmp |= (input[6].wrapping_sub(base)) << 4;
    tmp |= (input[7].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (22 - 16);
    tmp |= (input[8].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (22 - 6);
    tmp |= (input[9].wrapping_sub(base)) << 6;
    tmp |= (input[10].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (22 - 18);
    tmp |= (input[11].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (22 - 8);
    tmp |= (input[12].wrapping_sub(base)) << 8;
    tmp |= (input[13].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (22 - 20);
    tmp |= (input[14].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (22 - 10);
    tmp |= (input[15].wrapping_sub(base)) << 10;
    write_partial(output, out_off, tmp, 4);
    44
}

pub fn unpack22_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 4194303);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (22 - 12);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (22 - 2);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 2) & 4194303);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (22 - 14);
    output[4] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 14;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (22 - 4);
    output[5] = base.wrapping_add(tmp);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 4) & 4194303);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (22 - 16);
    output[7] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (22 - 6);
    output[8] = base.wrapping_add(tmp);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 6) & 4194303);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 18)) << (22 - 18);
    output[10] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (22 - 8);
    output[11] = base.wrapping_add(tmp);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 8) & 4194303);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (22 - 20);
    output[13] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (22 - 10);
    output[14] = base.wrapping_add(tmp);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 10) & 4194303);
    44
}

pub fn pack23_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 23;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (23 - 14);
    tmp |= (input[2].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (23 - 5);
    tmp |= (input[3].wrapping_sub(base)) << 5;
    tmp |= (input[4].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (23 - 19);
    tmp |= (input[5].wrapping_sub(base)) << 19;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (23 - 10);
    tmp |= (input[6].wrapping_sub(base)) << 10;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (23 - 1);
    tmp |= (input[7].wrapping_sub(base)) << 1;
    tmp |= (input[8].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (23 - 15);
    tmp |= (input[9].wrapping_sub(base)) << 15;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (23 - 6);
    tmp |= (input[10].wrapping_sub(base)) << 6;
    tmp |= (input[11].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (23 - 20);
    tmp |= (input[12].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (23 - 11);
    tmp |= (input[13].wrapping_sub(base)) << 11;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (23 - 2);
    tmp |= (input[14].wrapping_sub(base)) << 2;
    tmp |= (input[15].wrapping_sub(base)) << 25;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[15].wrapping_sub(base)) >> (23 - 16);
    write_partial(output, out_off, tmp, 2);
    46
}

pub fn unpack23_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 8388607);
    tmp = read_u32(input, in_off) >> 23;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (23 - 14);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 14;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 5)) << (23 - 5);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 5) & 8388607);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 19)) << (23 - 19);
    output[4] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 19;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (23 - 10);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 10;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (23 - 1);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 1) & 8388607);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 15)) << (23 - 15);
    output[8] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 15;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (23 - 6);
    output[9] = base.wrapping_add(tmp);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 6) & 8388607);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (23 - 20);
    output[11] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 11)) << (23 - 11);
    output[12] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 11;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (23 - 2);
    output[13] = base.wrapping_add(tmp);
    output[14] = base.wrapping_add((read_u32(input, in_off) >> 2) & 8388607);
    tmp = read_u32(input, in_off) >> 25;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (23 - 16);
    output[15] = base.wrapping_add(tmp);
    46
}

pub fn pack24_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (24 - 16);
    tmp |= (input[2].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (24 - 8);
    tmp |= (input[3].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) << 0;
    tmp |= (input[5].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (24 - 16);
    tmp |= (input[6].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (24 - 8);
    tmp |= (input[7].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) << 0;
    tmp |= (input[9].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (24 - 16);
    tmp |= (input[10].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (24 - 8);
    tmp |= (input[11].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) << 0;
    tmp |= (input[13].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (24 - 16);
    tmp |= (input[14].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (24 - 8);
    tmp |= (input[15].wrapping_sub(base)) << 8;
    write_partial(output, out_off, tmp, 4);
    48
}

pub fn unpack24_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 16777215);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (24 - 16);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (24 - 8);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 8) & 16777215);
    in_off += 4;
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 0) & 16777215);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (24 - 16);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (24 - 8);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 8) & 16777215);
    in_off += 4;
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 0) & 16777215);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (24 - 16);
    output[9] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (24 - 8);
    output[10] = base.wrapping_add(tmp);
    output[11] = base.wrapping_add((read_u32(input, in_off) >> 8) & 16777215);
    in_off += 4;
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 0) & 16777215);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (24 - 16);
    output[13] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (24 - 8);
    output[14] = base.wrapping_add(tmp);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 8) & 16777215);
    48
}

pub fn pack25_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 25;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (25 - 18);
    tmp |= (input[2].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (25 - 11);
    tmp |= (input[3].wrapping_sub(base)) << 11;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (25 - 4);
    tmp |= (input[4].wrapping_sub(base)) << 4;
    tmp |= (input[5].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (25 - 22);
    tmp |= (input[6].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (25 - 15);
    tmp |= (input[7].wrapping_sub(base)) << 15;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (25 - 8);
    tmp |= (input[8].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (25 - 1);
    tmp |= (input[9].wrapping_sub(base)) << 1;
    tmp |= (input[10].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (25 - 19);
    tmp |= (input[11].wrapping_sub(base)) << 19;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (25 - 12);
    tmp |= (input[12].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (25 - 5);
    tmp |= (input[13].wrapping_sub(base)) << 5;
    tmp |= (input[14].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (25 - 23);
    tmp |= (input[15].wrapping_sub(base)) << 23;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[15].wrapping_sub(base)) >> (25 - 16);
    write_partial(output, out_off, tmp, 2);
    50
}

pub fn unpack25_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 33554431);
    tmp = read_u32(input, in_off) >> 25;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 18)) << (25 - 18);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 11)) << (25 - 11);
    output[2] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 11;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (25 - 4);
    output[3] = base.wrapping_add(tmp);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 4) & 33554431);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 22)) << (25 - 22);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 15)) << (25 - 15);
    output[6] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 15;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (25 - 8);
    output[7] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 8;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (25 - 1);
    output[8] = base.wrapping_add(tmp);
    output[9] = base.wrapping_add((read_u32(input, in_off) >> 1) & 33554431);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 19)) << (25 - 19);
    output[10] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 19;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (25 - 12);
    output[11] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 5)) << (25 - 5);
    output[12] = base.wrapping_add(tmp);
    output[13] = base.wrapping_add((read_u32(input, in_off) >> 5) & 33554431);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 23)) << (25 - 23);
    output[14] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 23;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (25 - 16);
    output[15] = base.wrapping_add(tmp);
    50
}

pub fn pack26_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (26 - 20);
    tmp |= (input[2].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (26 - 14);
    tmp |= (input[3].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (26 - 8);
    tmp |= (input[4].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (26 - 2);
    tmp |= (input[5].wrapping_sub(base)) << 2;
    tmp |= (input[6].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (26 - 22);
    tmp |= (input[7].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (26 - 16);
    tmp |= (input[8].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (26 - 10);
    tmp |= (input[9].wrapping_sub(base)) << 10;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (26 - 4);
    tmp |= (input[10].wrapping_sub(base)) << 4;
    tmp |= (input[11].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (26 - 24);
    tmp |= (input[12].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (26 - 18);
    tmp |= (input[13].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (26 - 12);
    tmp |= (input[14].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (26 - 6);
    tmp |= (input[15].wrapping_sub(base)) << 6;
    write_partial(output, out_off, tmp, 4);
    52
}

pub fn unpack26_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 67108863);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (26 - 20);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (26 - 14);
    output[2] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 14;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (26 - 8);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 8;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (26 - 2);
    output[4] = base.wrapping_add(tmp);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 2) & 67108863);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 22)) << (26 - 22);
    output[6] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (26 - 16);
    output[7] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (26 - 10);
    output[8] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 10;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (26 - 4);
    output[9] = base.wrapping_add(tmp);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 4) & 67108863);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 24)) << (26 - 24);
    output[11] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 18)) << (26 - 18);
    output[12] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (26 - 12);
    output[13] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (26 - 6);
    output[14] = base.wrapping_add(tmp);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 6) & 67108863);
    52
}

pub fn pack27_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (27 - 22);
    tmp |= (input[2].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (27 - 17);
    tmp |= (input[3].wrapping_sub(base)) << 17;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (27 - 12);
    tmp |= (input[4].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (27 - 7);
    tmp |= (input[5].wrapping_sub(base)) << 7;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (27 - 2);
    tmp |= (input[6].wrapping_sub(base)) << 2;
    tmp |= (input[7].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (27 - 24);
    tmp |= (input[8].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (27 - 19);
    tmp |= (input[9].wrapping_sub(base)) << 19;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (27 - 14);
    tmp |= (input[10].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (27 - 9);
    tmp |= (input[11].wrapping_sub(base)) << 9;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (27 - 4);
    tmp |= (input[12].wrapping_sub(base)) << 4;
    tmp |= (input[13].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (27 - 26);
    tmp |= (input[14].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (27 - 21);
    tmp |= (input[15].wrapping_sub(base)) << 21;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[15].wrapping_sub(base)) >> (27 - 16);
    write_partial(output, out_off, tmp, 2);
    54
}

pub fn unpack27_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 134217727);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 22)) << (27 - 22);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 17)) << (27 - 17);
    output[2] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 17;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (27 - 12);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 7)) << (27 - 7);
    output[4] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 7;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (27 - 2);
    output[5] = base.wrapping_add(tmp);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 2) & 134217727);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 24)) << (27 - 24);
    output[7] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 19)) << (27 - 19);
    output[8] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 19;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (27 - 14);
    output[9] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 14;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 9)) << (27 - 9);
    output[10] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 9;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (27 - 4);
    output[11] = base.wrapping_add(tmp);
    output[12] = base.wrapping_add((read_u32(input, in_off) >> 4) & 134217727);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 26)) << (27 - 26);
    output[13] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 21)) << (27 - 21);
    output[14] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 21;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (27 - 16);
    output[15] = base.wrapping_add(tmp);
    54
}

pub fn pack28_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (28 - 24);
    tmp |= (input[2].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (28 - 20);
    tmp |= (input[3].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (28 - 16);
    tmp |= (input[4].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (28 - 12);
    tmp |= (input[5].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (28 - 8);
    tmp |= (input[6].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (28 - 4);
    tmp |= (input[7].wrapping_sub(base)) << 4;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) << 0;
    tmp |= (input[9].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (28 - 24);
    tmp |= (input[10].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (28 - 20);
    tmp |= (input[11].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (28 - 16);
    tmp |= (input[12].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (28 - 12);
    tmp |= (input[13].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (28 - 8);
    tmp |= (input[14].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (28 - 4);
    tmp |= (input[15].wrapping_sub(base)) << 4;
    write_partial(output, out_off, tmp, 4);
    56
}

pub fn unpack28_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 268435455);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 24)) << (28 - 24);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (28 - 20);
    output[2] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (28 - 16);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (28 - 12);
    output[4] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (28 - 8);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 8;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (28 - 4);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 4) & 268435455);
    in_off += 4;
    output[8] = base.wrapping_add((read_u32(input, in_off) >> 0) & 268435455);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 24)) << (28 - 24);
    output[9] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (28 - 20);
    output[10] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (28 - 16);
    output[11] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (28 - 12);
    output[12] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (28 - 8);
    output[13] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 8;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (28 - 4);
    output[14] = base.wrapping_add(tmp);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 4) & 268435455);
    56
}

pub fn pack29_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (29 - 26);
    tmp |= (input[2].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (29 - 23);
    tmp |= (input[3].wrapping_sub(base)) << 23;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (29 - 20);
    tmp |= (input[4].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (29 - 17);
    tmp |= (input[5].wrapping_sub(base)) << 17;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (29 - 14);
    tmp |= (input[6].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (29 - 11);
    tmp |= (input[7].wrapping_sub(base)) << 11;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (29 - 8);
    tmp |= (input[8].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (29 - 5);
    tmp |= (input[9].wrapping_sub(base)) << 5;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (29 - 2);
    tmp |= (input[10].wrapping_sub(base)) << 2;
    tmp |= (input[11].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (29 - 28);
    tmp |= (input[12].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (29 - 25);
    tmp |= (input[13].wrapping_sub(base)) << 25;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (29 - 22);
    tmp |= (input[14].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (29 - 19);
    tmp |= (input[15].wrapping_sub(base)) << 19;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[15].wrapping_sub(base)) >> (29 - 16);
    write_partial(output, out_off, tmp, 2);
    58
}

pub fn unpack29_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 536870911);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 26)) << (29 - 26);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 23)) << (29 - 23);
    output[2] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 23;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (29 - 20);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 17)) << (29 - 17);
    output[4] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 17;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (29 - 14);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 14;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 11)) << (29 - 11);
    output[6] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 11;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (29 - 8);
    output[7] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 8;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 5)) << (29 - 5);
    output[8] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 5;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (29 - 2);
    output[9] = base.wrapping_add(tmp);
    output[10] = base.wrapping_add((read_u32(input, in_off) >> 2) & 536870911);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 28)) << (29 - 28);
    output[11] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 25)) << (29 - 25);
    output[12] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 25;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 22)) << (29 - 22);
    output[13] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 19)) << (29 - 19);
    output[14] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 19;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (29 - 16);
    output[15] = base.wrapping_add(tmp);
    58
}

pub fn pack30_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (30 - 28);
    tmp |= (input[2].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (30 - 26);
    tmp |= (input[3].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (30 - 24);
    tmp |= (input[4].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (30 - 22);
    tmp |= (input[5].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (30 - 20);
    tmp |= (input[6].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (30 - 18);
    tmp |= (input[7].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (30 - 16);
    tmp |= (input[8].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (30 - 14);
    tmp |= (input[9].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (30 - 12);
    tmp |= (input[10].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (30 - 10);
    tmp |= (input[11].wrapping_sub(base)) << 10;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (30 - 8);
    tmp |= (input[12].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (30 - 6);
    tmp |= (input[13].wrapping_sub(base)) << 6;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (30 - 4);
    tmp |= (input[14].wrapping_sub(base)) << 4;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (30 - 2);
    tmp |= (input[15].wrapping_sub(base)) << 2;
    write_partial(output, out_off, tmp, 4);
    60
}

pub fn unpack30_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 1073741823);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 28)) << (30 - 28);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 26)) << (30 - 26);
    output[2] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 24)) << (30 - 24);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 22)) << (30 - 22);
    output[4] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (30 - 20);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 18)) << (30 - 18);
    output[6] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (30 - 16);
    output[7] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (30 - 14);
    output[8] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 14;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (30 - 12);
    output[9] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (30 - 10);
    output[10] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 10;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (30 - 8);
    output[11] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 8;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (30 - 6);
    output[12] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 6;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (30 - 4);
    output[13] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 4;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (30 - 2);
    output[14] = base.wrapping_add(tmp);
    output[15] = base.wrapping_add((read_u32(input, in_off) >> 2) & 1073741823);
    60
}

pub fn pack31_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (31 - 30);
    tmp |= (input[2].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (31 - 29);
    tmp |= (input[3].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (31 - 28);
    tmp |= (input[4].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (31 - 27);
    tmp |= (input[5].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (31 - 26);
    tmp |= (input[6].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (31 - 25);
    tmp |= (input[7].wrapping_sub(base)) << 25;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (31 - 24);
    tmp |= (input[8].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[8].wrapping_sub(base)) >> (31 - 23);
    tmp |= (input[9].wrapping_sub(base)) << 23;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[9].wrapping_sub(base)) >> (31 - 22);
    tmp |= (input[10].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[10].wrapping_sub(base)) >> (31 - 21);
    tmp |= (input[11].wrapping_sub(base)) << 21;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[11].wrapping_sub(base)) >> (31 - 20);
    tmp |= (input[12].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[12].wrapping_sub(base)) >> (31 - 19);
    tmp |= (input[13].wrapping_sub(base)) << 19;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[13].wrapping_sub(base)) >> (31 - 18);
    tmp |= (input[14].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[14].wrapping_sub(base)) >> (31 - 17);
    tmp |= (input[15].wrapping_sub(base)) << 17;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[15].wrapping_sub(base)) >> (31 - 16);
    write_partial(output, out_off, tmp, 2);
    62
}

pub fn unpack31_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 2147483647);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 30)) << (31 - 30);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 29)) << (31 - 29);
    output[2] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 28)) << (31 - 28);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 27)) << (31 - 27);
    output[4] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 26)) << (31 - 26);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 25)) << (31 - 25);
    output[6] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 25;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 24)) << (31 - 24);
    output[7] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 23)) << (31 - 23);
    output[8] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 23;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 22)) << (31 - 22);
    output[9] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 21)) << (31 - 21);
    output[10] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 21;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (31 - 20);
    output[11] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 19)) << (31 - 19);
    output[12] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 19;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 18)) << (31 - 18);
    output[13] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 17)) << (31 - 17);
    output[14] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 17;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (31 - 16);
    output[15] = base.wrapping_add(tmp);
    62
}

pub fn pack32_16(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    for i in 0..16 {
        write_u32(output, i * 4, input[i].wrapping_sub(base));
    }
    64
}

pub fn unpack32_16(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    for i in 0..16 {
        output[i] = base.wrapping_add(read_u32(input, i * 4));
    }
    64
}

pub fn pack1_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 1;
    tmp |= (input[2].wrapping_sub(base)) << 2;
    tmp |= (input[3].wrapping_sub(base)) << 3;
    tmp |= (input[4].wrapping_sub(base)) << 4;
    tmp |= (input[5].wrapping_sub(base)) << 5;
    tmp |= (input[6].wrapping_sub(base)) << 6;
    tmp |= (input[7].wrapping_sub(base)) << 7;
    write_partial(output, out_off, tmp, 1);
    1
}

pub fn unpack1_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 1);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 1) & 1);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 2) & 1);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 3) & 1);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 4) & 1);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 5) & 1);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 6) & 1);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 7) & 1);
    1
}

pub fn pack2_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 2;
    tmp |= (input[2].wrapping_sub(base)) << 4;
    tmp |= (input[3].wrapping_sub(base)) << 6;
    tmp |= (input[4].wrapping_sub(base)) << 8;
    tmp |= (input[5].wrapping_sub(base)) << 10;
    tmp |= (input[6].wrapping_sub(base)) << 12;
    tmp |= (input[7].wrapping_sub(base)) << 14;
    write_partial(output, out_off, tmp, 2);
    2
}

pub fn unpack2_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 3);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 2) & 3);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 4) & 3);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 6) & 3);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 8) & 3);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 10) & 3);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 12) & 3);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 14) & 3);
    2
}

pub fn pack3_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 3;
    tmp |= (input[2].wrapping_sub(base)) << 6;
    tmp |= (input[3].wrapping_sub(base)) << 9;
    tmp |= (input[4].wrapping_sub(base)) << 12;
    tmp |= (input[5].wrapping_sub(base)) << 15;
    tmp |= (input[6].wrapping_sub(base)) << 18;
    tmp |= (input[7].wrapping_sub(base)) << 21;
    write_partial(output, out_off, tmp, 3);
    3
}

pub fn unpack3_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 7);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 3) & 7);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 6) & 7);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 9) & 7);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 12) & 7);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 15) & 7);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 18) & 7);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 21) & 7);
    3
}

pub fn pack4_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 4;
    tmp |= (input[2].wrapping_sub(base)) << 8;
    tmp |= (input[3].wrapping_sub(base)) << 12;
    tmp |= (input[4].wrapping_sub(base)) << 16;
    tmp |= (input[5].wrapping_sub(base)) << 20;
    tmp |= (input[6].wrapping_sub(base)) << 24;
    tmp |= (input[7].wrapping_sub(base)) << 28;
    write_partial(output, out_off, tmp, 4);
    4
}

pub fn unpack4_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 15);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 4) & 15);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 8) & 15);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 12) & 15);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 16) & 15);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 20) & 15);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 24) & 15);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 28) & 15);
    4
}

pub fn pack5_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 5;
    tmp |= (input[2].wrapping_sub(base)) << 10;
    tmp |= (input[3].wrapping_sub(base)) << 15;
    tmp |= (input[4].wrapping_sub(base)) << 20;
    tmp |= (input[5].wrapping_sub(base)) << 25;
    tmp |= (input[6].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (5 - 3);
    tmp |= (input[7].wrapping_sub(base)) << 3;
    write_partial(output, out_off, tmp, 1);
    5
}

pub fn unpack5_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 31);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 5) & 31);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 10) & 31);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 15) & 31);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 20) & 31);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 25) & 31);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 3)) << (5 - 3);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 3) & 31);
    5
}

pub fn pack6_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 6;
    tmp |= (input[2].wrapping_sub(base)) << 12;
    tmp |= (input[3].wrapping_sub(base)) << 18;
    tmp |= (input[4].wrapping_sub(base)) << 24;
    tmp |= (input[5].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (6 - 4);
    tmp |= (input[6].wrapping_sub(base)) << 4;
    tmp |= (input[7].wrapping_sub(base)) << 10;
    write_partial(output, out_off, tmp, 2);
    6
}

pub fn unpack6_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 63);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 6) & 63);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 12) & 63);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 18) & 63);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 24) & 63);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (6 - 4);
    output[5] = base.wrapping_add(tmp);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 4) & 63);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 10) & 63);
    6
}

pub fn pack7_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 7;
    tmp |= (input[2].wrapping_sub(base)) << 14;
    tmp |= (input[3].wrapping_sub(base)) << 21;
    tmp |= (input[4].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (7 - 3);
    tmp |= (input[5].wrapping_sub(base)) << 3;
    tmp |= (input[6].wrapping_sub(base)) << 10;
    tmp |= (input[7].wrapping_sub(base)) << 17;
    write_partial(output, out_off, tmp, 3);
    7
}

pub fn unpack7_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 127);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 7) & 127);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 14) & 127);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 21) & 127);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 3)) << (7 - 3);
    output[4] = base.wrapping_add(tmp);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 3) & 127);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 10) & 127);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 17) & 127);
    7
}

pub fn pack8_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 8;
    tmp |= (input[2].wrapping_sub(base)) << 16;
    tmp |= (input[3].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) << 0;
    tmp |= (input[5].wrapping_sub(base)) << 8;
    tmp |= (input[6].wrapping_sub(base)) << 16;
    tmp |= (input[7].wrapping_sub(base)) << 24;
    write_partial(output, out_off, tmp, 4);
    8
}

pub fn unpack8_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 255);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 8) & 255);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 16) & 255);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 24) & 255);
    in_off += 4;
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 0) & 255);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 8) & 255);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 16) & 255);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 24) & 255);
    8
}

pub fn pack9_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 9;
    tmp |= (input[2].wrapping_sub(base)) << 18;
    tmp |= (input[3].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (9 - 4);
    tmp |= (input[4].wrapping_sub(base)) << 4;
    tmp |= (input[5].wrapping_sub(base)) << 13;
    tmp |= (input[6].wrapping_sub(base)) << 22;
    tmp |= (input[7].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (9 - 8);
    write_partial(output, out_off, tmp, 1);
    9
}

pub fn unpack9_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 511);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 9) & 511);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 18) & 511);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (9 - 4);
    output[3] = base.wrapping_add(tmp);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 4) & 511);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 13) & 511);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 22) & 511);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (9 - 8);
    output[7] = base.wrapping_add(tmp);
    9
}

pub fn pack10_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 10;
    tmp |= (input[2].wrapping_sub(base)) << 20;
    tmp |= (input[3].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (10 - 8);
    tmp |= (input[4].wrapping_sub(base)) << 8;
    tmp |= (input[5].wrapping_sub(base)) << 18;
    tmp |= (input[6].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (10 - 6);
    tmp |= (input[7].wrapping_sub(base)) << 6;
    write_partial(output, out_off, tmp, 2);
    10
}

pub fn unpack10_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 1023);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 10) & 1023);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 20) & 1023);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (10 - 8);
    output[3] = base.wrapping_add(tmp);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 8) & 1023);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 18) & 1023);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (10 - 6);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 6) & 1023);
    10
}

pub fn pack11_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 11;
    tmp |= (input[2].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (11 - 1);
    tmp |= (input[3].wrapping_sub(base)) << 1;
    tmp |= (input[4].wrapping_sub(base)) << 12;
    tmp |= (input[5].wrapping_sub(base)) << 23;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (11 - 2);
    tmp |= (input[6].wrapping_sub(base)) << 2;
    tmp |= (input[7].wrapping_sub(base)) << 13;
    write_partial(output, out_off, tmp, 3);
    11
}

pub fn unpack11_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 2047);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 11) & 2047);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (11 - 1);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 1) & 2047);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 12) & 2047);
    tmp = read_u32(input, in_off) >> 23;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (11 - 2);
    output[5] = base.wrapping_add(tmp);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 2) & 2047);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 13) & 2047);
    11
}

pub fn pack12_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 12;
    tmp |= (input[2].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (12 - 4);
    tmp |= (input[3].wrapping_sub(base)) << 4;
    tmp |= (input[4].wrapping_sub(base)) << 16;
    tmp |= (input[5].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (12 - 8);
    tmp |= (input[6].wrapping_sub(base)) << 8;
    tmp |= (input[7].wrapping_sub(base)) << 20;
    write_partial(output, out_off, tmp, 4);
    12
}

pub fn unpack12_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 4095);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 12) & 4095);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (12 - 4);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 4) & 4095);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 16) & 4095);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (12 - 8);
    output[5] = base.wrapping_add(tmp);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 8) & 4095);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 20) & 4095);
    12
}

pub fn pack13_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 13;
    tmp |= (input[2].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (13 - 7);
    tmp |= (input[3].wrapping_sub(base)) << 7;
    tmp |= (input[4].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (13 - 1);
    tmp |= (input[5].wrapping_sub(base)) << 1;
    tmp |= (input[6].wrapping_sub(base)) << 14;
    tmp |= (input[7].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (13 - 8);
    write_partial(output, out_off, tmp, 1);
    13
}

pub fn unpack13_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 8191);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 13) & 8191);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 7)) << (13 - 7);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 7) & 8191);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (13 - 1);
    output[4] = base.wrapping_add(tmp);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 1) & 8191);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 14) & 8191);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (13 - 8);
    output[7] = base.wrapping_add(tmp);
    13
}

pub fn pack14_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 14;
    tmp |= (input[2].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (14 - 10);
    tmp |= (input[3].wrapping_sub(base)) << 10;
    tmp |= (input[4].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (14 - 6);
    tmp |= (input[5].wrapping_sub(base)) << 6;
    tmp |= (input[6].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (14 - 2);
    tmp |= (input[7].wrapping_sub(base)) << 2;
    write_partial(output, out_off, tmp, 2);
    14
}

pub fn unpack14_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 16383);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 14) & 16383);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (14 - 10);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 10) & 16383);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (14 - 6);
    output[4] = base.wrapping_add(tmp);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 6) & 16383);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (14 - 2);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 2) & 16383);
    14
}

pub fn pack15_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 15;
    tmp |= (input[2].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (15 - 13);
    tmp |= (input[3].wrapping_sub(base)) << 13;
    tmp |= (input[4].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (15 - 11);
    tmp |= (input[5].wrapping_sub(base)) << 11;
    tmp |= (input[6].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (15 - 9);
    tmp |= (input[7].wrapping_sub(base)) << 9;
    write_partial(output, out_off, tmp, 3);
    15
}

pub fn unpack15_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 32767);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 15) & 32767);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 13)) << (15 - 13);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 13) & 32767);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 11)) << (15 - 11);
    output[4] = base.wrapping_add(tmp);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 11) & 32767);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 9)) << (15 - 9);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 9) & 32767);
    15
}

pub fn pack16_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) << 0;
    tmp |= (input[3].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) << 0;
    tmp |= (input[5].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) << 0;
    tmp |= (input[7].wrapping_sub(base)) << 16;
    write_partial(output, out_off, tmp, 4);
    16
}

pub fn unpack16_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[1] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    in_off += 4;
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 0) & 65535);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 16) & 65535);
    16
}

pub fn pack17_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 17;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (17 - 2);
    tmp |= (input[2].wrapping_sub(base)) << 2;
    tmp |= (input[3].wrapping_sub(base)) << 19;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (17 - 4);
    tmp |= (input[4].wrapping_sub(base)) << 4;
    tmp |= (input[5].wrapping_sub(base)) << 21;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (17 - 6);
    tmp |= (input[6].wrapping_sub(base)) << 6;
    tmp |= (input[7].wrapping_sub(base)) << 23;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (17 - 8);
    write_partial(output, out_off, tmp, 1);
    17
}

pub fn unpack17_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 131071);
    tmp = read_u32(input, in_off) >> 17;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (17 - 2);
    output[1] = base.wrapping_add(tmp);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 2) & 131071);
    tmp = read_u32(input, in_off) >> 19;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (17 - 4);
    output[3] = base.wrapping_add(tmp);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 4) & 131071);
    tmp = read_u32(input, in_off) >> 21;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (17 - 6);
    output[5] = base.wrapping_add(tmp);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 6) & 131071);
    tmp = read_u32(input, in_off) >> 23;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (17 - 8);
    output[7] = base.wrapping_add(tmp);
    17
}

pub fn pack18_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (18 - 4);
    tmp |= (input[2].wrapping_sub(base)) << 4;
    tmp |= (input[3].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (18 - 8);
    tmp |= (input[4].wrapping_sub(base)) << 8;
    tmp |= (input[5].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (18 - 12);
    tmp |= (input[6].wrapping_sub(base)) << 12;
    tmp |= (input[7].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (18 - 16);
    write_partial(output, out_off, tmp, 2);
    18
}

pub fn unpack18_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 262143);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (18 - 4);
    output[1] = base.wrapping_add(tmp);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 4) & 262143);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (18 - 8);
    output[3] = base.wrapping_add(tmp);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 8) & 262143);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (18 - 12);
    output[5] = base.wrapping_add(tmp);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 12) & 262143);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (18 - 16);
    output[7] = base.wrapping_add(tmp);
    18
}

pub fn pack19_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 19;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (19 - 6);
    tmp |= (input[2].wrapping_sub(base)) << 6;
    tmp |= (input[3].wrapping_sub(base)) << 25;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (19 - 12);
    tmp |= (input[4].wrapping_sub(base)) << 12;
    tmp |= (input[5].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (19 - 18);
    tmp |= (input[6].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (19 - 5);
    tmp |= (input[7].wrapping_sub(base)) << 5;
    write_partial(output, out_off, tmp, 3);
    19
}

pub fn unpack19_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 524287);
    tmp = read_u32(input, in_off) >> 19;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 6)) << (19 - 6);
    output[1] = base.wrapping_add(tmp);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 6) & 524287);
    tmp = read_u32(input, in_off) >> 25;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (19 - 12);
    output[3] = base.wrapping_add(tmp);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 12) & 524287);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 18)) << (19 - 18);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 5)) << (19 - 5);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 5) & 524287);
    19
}

pub fn pack20_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (20 - 8);
    tmp |= (input[2].wrapping_sub(base)) << 8;
    tmp |= (input[3].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (20 - 16);
    tmp |= (input[4].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (20 - 4);
    tmp |= (input[5].wrapping_sub(base)) << 4;
    tmp |= (input[6].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (20 - 12);
    tmp |= (input[7].wrapping_sub(base)) << 12;
    write_partial(output, out_off, tmp, 4);
    20
}

pub fn unpack20_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 1048575);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (20 - 8);
    output[1] = base.wrapping_add(tmp);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 8) & 1048575);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (20 - 16);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (20 - 4);
    output[4] = base.wrapping_add(tmp);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 4) & 1048575);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (20 - 12);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 12) & 1048575);
    20
}

pub fn pack21_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 21;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (21 - 10);
    tmp |= (input[2].wrapping_sub(base)) << 10;
    tmp |= (input[3].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (21 - 20);
    tmp |= (input[4].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (21 - 9);
    tmp |= (input[5].wrapping_sub(base)) << 9;
    tmp |= (input[6].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (21 - 19);
    tmp |= (input[7].wrapping_sub(base)) << 19;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (21 - 8);
    write_partial(output, out_off, tmp, 1);
    21
}

pub fn unpack21_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 2097151);
    tmp = read_u32(input, in_off) >> 21;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (21 - 10);
    output[1] = base.wrapping_add(tmp);
    output[2] = base.wrapping_add((read_u32(input, in_off) >> 10) & 2097151);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (21 - 20);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 9)) << (21 - 9);
    output[4] = base.wrapping_add(tmp);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 9) & 2097151);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 19)) << (21 - 19);
    output[6] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 19;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (21 - 8);
    output[7] = base.wrapping_add(tmp);
    21
}

pub fn pack22_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (22 - 12);
    tmp |= (input[2].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (22 - 2);
    tmp |= (input[3].wrapping_sub(base)) << 2;
    tmp |= (input[4].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (22 - 14);
    tmp |= (input[5].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (22 - 4);
    tmp |= (input[6].wrapping_sub(base)) << 4;
    tmp |= (input[7].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (22 - 16);
    write_partial(output, out_off, tmp, 2);
    22
}

pub fn unpack22_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 4194303);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (22 - 12);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (22 - 2);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 2) & 4194303);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (22 - 14);
    output[4] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 14;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (22 - 4);
    output[5] = base.wrapping_add(tmp);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 4) & 4194303);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (22 - 16);
    output[7] = base.wrapping_add(tmp);
    22
}

pub fn pack23_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 23;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (23 - 14);
    tmp |= (input[2].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (23 - 5);
    tmp |= (input[3].wrapping_sub(base)) << 5;
    tmp |= (input[4].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (23 - 19);
    tmp |= (input[5].wrapping_sub(base)) << 19;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (23 - 10);
    tmp |= (input[6].wrapping_sub(base)) << 10;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (23 - 1);
    tmp |= (input[7].wrapping_sub(base)) << 1;
    write_partial(output, out_off, tmp, 3);
    23
}

pub fn unpack23_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 8388607);
    tmp = read_u32(input, in_off) >> 23;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (23 - 14);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 14;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 5)) << (23 - 5);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 5) & 8388607);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 19)) << (23 - 19);
    output[4] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 19;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 10)) << (23 - 10);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 10;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 1)) << (23 - 1);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 1) & 8388607);
    23
}

pub fn pack24_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (24 - 16);
    tmp |= (input[2].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (24 - 8);
    tmp |= (input[3].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) << 0;
    tmp |= (input[5].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (24 - 16);
    tmp |= (input[6].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (24 - 8);
    tmp |= (input[7].wrapping_sub(base)) << 8;
    write_partial(output, out_off, tmp, 4);
    24
}

pub fn unpack24_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 16777215);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (24 - 16);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (24 - 8);
    output[2] = base.wrapping_add(tmp);
    output[3] = base.wrapping_add((read_u32(input, in_off) >> 8) & 16777215);
    in_off += 4;
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 0) & 16777215);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (24 - 16);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (24 - 8);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 8) & 16777215);
    24
}

pub fn pack25_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 25;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (25 - 18);
    tmp |= (input[2].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (25 - 11);
    tmp |= (input[3].wrapping_sub(base)) << 11;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (25 - 4);
    tmp |= (input[4].wrapping_sub(base)) << 4;
    tmp |= (input[5].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (25 - 22);
    tmp |= (input[6].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (25 - 15);
    tmp |= (input[7].wrapping_sub(base)) << 15;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (25 - 8);
    write_partial(output, out_off, tmp, 1);
    25
}

pub fn unpack25_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 33554431);
    tmp = read_u32(input, in_off) >> 25;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 18)) << (25 - 18);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 11)) << (25 - 11);
    output[2] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 11;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (25 - 4);
    output[3] = base.wrapping_add(tmp);
    output[4] = base.wrapping_add((read_u32(input, in_off) >> 4) & 33554431);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 22)) << (25 - 22);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 15)) << (25 - 15);
    output[6] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 15;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (25 - 8);
    output[7] = base.wrapping_add(tmp);
    25
}

pub fn pack26_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (26 - 20);
    tmp |= (input[2].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (26 - 14);
    tmp |= (input[3].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (26 - 8);
    tmp |= (input[4].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (26 - 2);
    tmp |= (input[5].wrapping_sub(base)) << 2;
    tmp |= (input[6].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (26 - 22);
    tmp |= (input[7].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (26 - 16);
    write_partial(output, out_off, tmp, 2);
    26
}

pub fn unpack26_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 67108863);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (26 - 20);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (26 - 14);
    output[2] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 14;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (26 - 8);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 8;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (26 - 2);
    output[4] = base.wrapping_add(tmp);
    output[5] = base.wrapping_add((read_u32(input, in_off) >> 2) & 67108863);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 22)) << (26 - 22);
    output[6] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (26 - 16);
    output[7] = base.wrapping_add(tmp);
    26
}

pub fn pack27_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (27 - 22);
    tmp |= (input[2].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (27 - 17);
    tmp |= (input[3].wrapping_sub(base)) << 17;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (27 - 12);
    tmp |= (input[4].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (27 - 7);
    tmp |= (input[5].wrapping_sub(base)) << 7;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (27 - 2);
    tmp |= (input[6].wrapping_sub(base)) << 2;
    tmp |= (input[7].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (27 - 24);
    write_partial(output, out_off, tmp, 3);
    27
}

pub fn unpack27_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 134217727);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 22)) << (27 - 22);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 17)) << (27 - 17);
    output[2] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 17;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (27 - 12);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 7)) << (27 - 7);
    output[4] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 7;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 2)) << (27 - 2);
    output[5] = base.wrapping_add(tmp);
    output[6] = base.wrapping_add((read_u32(input, in_off) >> 2) & 134217727);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 24)) << (27 - 24);
    output[7] = base.wrapping_add(tmp);
    27
}

pub fn pack28_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (28 - 24);
    tmp |= (input[2].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (28 - 20);
    tmp |= (input[3].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (28 - 16);
    tmp |= (input[4].wrapping_sub(base)) << 16;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (28 - 12);
    tmp |= (input[5].wrapping_sub(base)) << 12;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (28 - 8);
    tmp |= (input[6].wrapping_sub(base)) << 8;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (28 - 4);
    tmp |= (input[7].wrapping_sub(base)) << 4;
    write_partial(output, out_off, tmp, 4);
    28
}

pub fn unpack28_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 268435455);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 24)) << (28 - 24);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (28 - 20);
    output[2] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (28 - 16);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 16;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 12)) << (28 - 12);
    output[4] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 12;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (28 - 8);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 8;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 4)) << (28 - 4);
    output[6] = base.wrapping_add(tmp);
    output[7] = base.wrapping_add((read_u32(input, in_off) >> 4) & 268435455);
    28
}

pub fn pack29_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (29 - 26);
    tmp |= (input[2].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (29 - 23);
    tmp |= (input[3].wrapping_sub(base)) << 23;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (29 - 20);
    tmp |= (input[4].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (29 - 17);
    tmp |= (input[5].wrapping_sub(base)) << 17;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (29 - 14);
    tmp |= (input[6].wrapping_sub(base)) << 14;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (29 - 11);
    tmp |= (input[7].wrapping_sub(base)) << 11;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (29 - 8);
    write_partial(output, out_off, tmp, 1);
    29
}

pub fn unpack29_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 536870911);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 26)) << (29 - 26);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 23)) << (29 - 23);
    output[2] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 23;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (29 - 20);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 17)) << (29 - 17);
    output[4] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 17;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 14)) << (29 - 14);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 14;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 11)) << (29 - 11);
    output[6] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 11;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 8)) << (29 - 8);
    output[7] = base.wrapping_add(tmp);
    29
}

pub fn pack30_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (30 - 28);
    tmp |= (input[2].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (30 - 26);
    tmp |= (input[3].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (30 - 24);
    tmp |= (input[4].wrapping_sub(base)) << 24;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (30 - 22);
    tmp |= (input[5].wrapping_sub(base)) << 22;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (30 - 20);
    tmp |= (input[6].wrapping_sub(base)) << 20;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (30 - 18);
    tmp |= (input[7].wrapping_sub(base)) << 18;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (30 - 16);
    write_partial(output, out_off, tmp, 2);
    30
}

pub fn unpack30_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 1073741823);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 28)) << (30 - 28);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 26)) << (30 - 26);
    output[2] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 24)) << (30 - 24);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 24;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 22)) << (30 - 22);
    output[4] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 22;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 20)) << (30 - 20);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 20;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 18)) << (30 - 18);
    output[6] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 18;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 16)) << (30 - 16);
    output[7] = base.wrapping_add(tmp);
    30
}

pub fn pack31_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    let mut tmp: u32;
    let mut out_off: usize = 0;
    tmp = (input[0].wrapping_sub(base)) << 0;
    tmp |= (input[1].wrapping_sub(base)) << 31;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[1].wrapping_sub(base)) >> (31 - 30);
    tmp |= (input[2].wrapping_sub(base)) << 30;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[2].wrapping_sub(base)) >> (31 - 29);
    tmp |= (input[3].wrapping_sub(base)) << 29;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[3].wrapping_sub(base)) >> (31 - 28);
    tmp |= (input[4].wrapping_sub(base)) << 28;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[4].wrapping_sub(base)) >> (31 - 27);
    tmp |= (input[5].wrapping_sub(base)) << 27;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[5].wrapping_sub(base)) >> (31 - 26);
    tmp |= (input[6].wrapping_sub(base)) << 26;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[6].wrapping_sub(base)) >> (31 - 25);
    tmp |= (input[7].wrapping_sub(base)) << 25;
    write_u32(output, out_off, tmp);
    out_off += 4;
    tmp = (input[7].wrapping_sub(base)) >> (31 - 24);
    write_partial(output, out_off, tmp, 3);
    31
}

pub fn unpack31_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    let mut in_off: usize = 0;
    let mut tmp: u32;
    output[0] = base.wrapping_add((read_u32(input, in_off) >> 0) & 2147483647);
    tmp = read_u32(input, in_off) >> 31;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 30)) << (31 - 30);
    output[1] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 30;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 29)) << (31 - 29);
    output[2] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 29;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 28)) << (31 - 28);
    output[3] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 28;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 27)) << (31 - 27);
    output[4] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 27;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 26)) << (31 - 26);
    output[5] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 26;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 25)) << (31 - 25);
    output[6] = base.wrapping_add(tmp);
    tmp = read_u32(input, in_off) >> 25;
    in_off += 4;
    tmp |= (read_u32(input, in_off) % (1u32 << 24)) << (31 - 24);
    output[7] = base.wrapping_add(tmp);
    31
}

pub fn pack32_8(base: u32, input: &[u32], output: &mut [u8]) -> u32 {
    for i in 0..8 {
        write_u32(output, i * 4, input[i].wrapping_sub(base));
    }
    32
}

pub fn unpack32_8(base: u32, input: &[u8], output: &mut [u32]) -> u32 {
    for i in 0..8 {
        output[i] = base.wrapping_add(read_u32(input, i * 4));
    }
    32
}

pub fn pack1_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 1;
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 2;
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 3;
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 4;
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 5;
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 6;
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 7;
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 1 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack1_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 1);
        if length == 1 { break 'bail; }
        output[1] = base.wrapping_add((tmp >> 1) & 1);
        if length == 2 { break 'bail; }
        output[2] = base.wrapping_add((tmp >> 2) & 1);
        if length == 3 { break 'bail; }
        output[3] = base.wrapping_add((tmp >> 3) & 1);
        if length == 4 { break 'bail; }
        output[4] = base.wrapping_add((tmp >> 4) & 1);
        if length == 5 { break 'bail; }
        output[5] = base.wrapping_add((tmp >> 5) & 1);
        if length == 6 { break 'bail; }
        output[6] = base.wrapping_add((tmp >> 6) & 1);
        if length == 7 { break 'bail; }
        output[7] = base.wrapping_add((tmp >> 7) & 1);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 1 + 7) / 8
}

pub fn pack2_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 2;
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 4;
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 6;
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 8;
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 10;
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 12;
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 14;
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 2 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack2_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 3);
        if length == 1 { break 'bail; }
        output[1] = base.wrapping_add((tmp >> 2) & 3);
        if length == 2 { break 'bail; }
        output[2] = base.wrapping_add((tmp >> 4) & 3);
        if length == 3 { break 'bail; }
        output[3] = base.wrapping_add((tmp >> 6) & 3);
        if length == 4 { break 'bail; }
        output[4] = base.wrapping_add((tmp >> 8) & 3);
        if length == 5 { break 'bail; }
        output[5] = base.wrapping_add((tmp >> 10) & 3);
        if length == 6 { break 'bail; }
        output[6] = base.wrapping_add((tmp >> 12) & 3);
        if length == 7 { break 'bail; }
        output[7] = base.wrapping_add((tmp >> 14) & 3);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 2 + 7) / 8
}

pub fn pack3_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 3;
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 6;
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 9;
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 12;
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 15;
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 18;
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 21;
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 3 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack3_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 7);
        if length == 1 { break 'bail; }
        output[1] = base.wrapping_add((tmp >> 3) & 7);
        if length == 2 { break 'bail; }
        output[2] = base.wrapping_add((tmp >> 6) & 7);
        if length == 3 { break 'bail; }
        output[3] = base.wrapping_add((tmp >> 9) & 7);
        if length == 4 { break 'bail; }
        output[4] = base.wrapping_add((tmp >> 12) & 7);
        if length == 5 { break 'bail; }
        output[5] = base.wrapping_add((tmp >> 15) & 7);
        if length == 6 { break 'bail; }
        output[6] = base.wrapping_add((tmp >> 18) & 7);
        if length == 7 { break 'bail; }
        output[7] = base.wrapping_add((tmp >> 21) & 7);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 3 + 7) / 8
}

pub fn pack4_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 4;
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 8;
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 12;
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 16;
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 20;
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 24;
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 28;
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 4 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack4_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 15);
        if length == 1 { break 'bail; }
        output[1] = base.wrapping_add((tmp >> 4) & 15);
        if length == 2 { break 'bail; }
        output[2] = base.wrapping_add((tmp >> 8) & 15);
        if length == 3 { break 'bail; }
        output[3] = base.wrapping_add((tmp >> 12) & 15);
        if length == 4 { break 'bail; }
        output[4] = base.wrapping_add((tmp >> 16) & 15);
        if length == 5 { break 'bail; }
        output[5] = base.wrapping_add((tmp >> 20) & 15);
        if length == 6 { break 'bail; }
        output[6] = base.wrapping_add((tmp >> 24) & 15);
        if length == 7 { break 'bail; }
        output[7] = base.wrapping_add((tmp >> 28) & 15);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 4 + 7) / 8
}

pub fn pack5_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 5;
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 10;
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 15;
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 20;
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 25;
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 30;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[6].wrapping_sub(base)) >> (5 - 3);
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 3;
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 5 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack5_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 31);
        if length == 1 { break 'bail; }
        output[1] = base.wrapping_add((tmp >> 5) & 31);
        if length == 2 { break 'bail; }
        output[2] = base.wrapping_add((tmp >> 10) & 31);
        if length == 3 { break 'bail; }
        output[3] = base.wrapping_add((tmp >> 15) & 31);
        if length == 4 { break 'bail; }
        output[4] = base.wrapping_add((tmp >> 20) & 31);
        if length == 5 { break 'bail; }
        output[5] = base.wrapping_add((tmp >> 25) & 31);
        if length == 6 { break 'bail; }
        output[6] = tmp >> 30;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[6] |= (tmp % (1u32 << 3)) << (5 - 3);
        output[6] = output[6].wrapping_add(base);
        if length == 7 { break 'bail; }
        output[7] = base.wrapping_add((tmp >> 3) & 31);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 5 + 7) / 8
}

pub fn pack6_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 6;
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 12;
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 18;
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 24;
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 30;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[5].wrapping_sub(base)) >> (6 - 4);
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 4;
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 10;
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 6 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack6_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 63);
        if length == 1 { break 'bail; }
        output[1] = base.wrapping_add((tmp >> 6) & 63);
        if length == 2 { break 'bail; }
        output[2] = base.wrapping_add((tmp >> 12) & 63);
        if length == 3 { break 'bail; }
        output[3] = base.wrapping_add((tmp >> 18) & 63);
        if length == 4 { break 'bail; }
        output[4] = base.wrapping_add((tmp >> 24) & 63);
        if length == 5 { break 'bail; }
        output[5] = tmp >> 30;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[5] |= (tmp % (1u32 << 4)) << (6 - 4);
        output[5] = output[5].wrapping_add(base);
        if length == 6 { break 'bail; }
        output[6] = base.wrapping_add((tmp >> 4) & 63);
        if length == 7 { break 'bail; }
        output[7] = base.wrapping_add((tmp >> 10) & 63);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 6 + 7) / 8
}

pub fn pack7_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 7;
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 14;
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 21;
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 28;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[4].wrapping_sub(base)) >> (7 - 3);
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 3;
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 10;
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 17;
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 7 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack7_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 127);
        if length == 1 { break 'bail; }
        output[1] = base.wrapping_add((tmp >> 7) & 127);
        if length == 2 { break 'bail; }
        output[2] = base.wrapping_add((tmp >> 14) & 127);
        if length == 3 { break 'bail; }
        output[3] = base.wrapping_add((tmp >> 21) & 127);
        if length == 4 { break 'bail; }
        output[4] = tmp >> 28;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[4] |= (tmp % (1u32 << 3)) << (7 - 3);
        output[4] = output[4].wrapping_add(base);
        if length == 5 { break 'bail; }
        output[5] = base.wrapping_add((tmp >> 3) & 127);
        if length == 6 { break 'bail; }
        output[6] = base.wrapping_add((tmp >> 10) & 127);
        if length == 7 { break 'bail; }
        output[7] = base.wrapping_add((tmp >> 17) & 127);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 7 + 7) / 8
}

pub fn pack8_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 8;
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 16;
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 24;
        if length == 4 { break 'bail; }
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[4].wrapping_sub(base)) << 0;
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 8;
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 16;
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 24;
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 8 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack8_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 255);
        if length == 1 { break 'bail; }
        output[1] = base.wrapping_add((tmp >> 8) & 255);
        if length == 2 { break 'bail; }
        output[2] = base.wrapping_add((tmp >> 16) & 255);
        if length == 3 { break 'bail; }
        output[3] = base.wrapping_add((tmp >> 24) & 255);
        if length == 4 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[4] = base.wrapping_add((tmp >> 0) & 255);
        if length == 5 { break 'bail; }
        output[5] = base.wrapping_add((tmp >> 8) & 255);
        if length == 6 { break 'bail; }
        output[6] = base.wrapping_add((tmp >> 16) & 255);
        if length == 7 { break 'bail; }
        output[7] = base.wrapping_add((tmp >> 24) & 255);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 8 + 7) / 8
}

pub fn pack9_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 9;
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 18;
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 27;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[3].wrapping_sub(base)) >> (9 - 4);
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 4;
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 13;
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 22;
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 31;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[7].wrapping_sub(base)) >> (9 - 8);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 9 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack9_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 511);
        if length == 1 { break 'bail; }
        output[1] = base.wrapping_add((tmp >> 9) & 511);
        if length == 2 { break 'bail; }
        output[2] = base.wrapping_add((tmp >> 18) & 511);
        if length == 3 { break 'bail; }
        output[3] = tmp >> 27;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[3] |= (tmp % (1u32 << 4)) << (9 - 4);
        output[3] = output[3].wrapping_add(base);
        if length == 4 { break 'bail; }
        output[4] = base.wrapping_add((tmp >> 4) & 511);
        if length == 5 { break 'bail; }
        output[5] = base.wrapping_add((tmp >> 13) & 511);
        if length == 6 { break 'bail; }
        output[6] = base.wrapping_add((tmp >> 22) & 511);
        if length == 7 { break 'bail; }
        output[7] = tmp >> 31;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[7] |= (tmp % (1u32 << 8)) << (9 - 8);
        output[7] = output[7].wrapping_add(base);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 9 + 7) / 8
}

pub fn pack10_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 10;
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 20;
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 30;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[3].wrapping_sub(base)) >> (10 - 8);
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 8;
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 18;
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 28;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[6].wrapping_sub(base)) >> (10 - 6);
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 6;
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 10 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack10_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 1023);
        if length == 1 { break 'bail; }
        output[1] = base.wrapping_add((tmp >> 10) & 1023);
        if length == 2 { break 'bail; }
        output[2] = base.wrapping_add((tmp >> 20) & 1023);
        if length == 3 { break 'bail; }
        output[3] = tmp >> 30;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[3] |= (tmp % (1u32 << 8)) << (10 - 8);
        output[3] = output[3].wrapping_add(base);
        if length == 4 { break 'bail; }
        output[4] = base.wrapping_add((tmp >> 8) & 1023);
        if length == 5 { break 'bail; }
        output[5] = base.wrapping_add((tmp >> 18) & 1023);
        if length == 6 { break 'bail; }
        output[6] = tmp >> 28;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[6] |= (tmp % (1u32 << 6)) << (10 - 6);
        output[6] = output[6].wrapping_add(base);
        if length == 7 { break 'bail; }
        output[7] = base.wrapping_add((tmp >> 6) & 1023);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 10 + 7) / 8
}

pub fn pack11_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 11;
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 22;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[2].wrapping_sub(base)) >> (11 - 1);
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 1;
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 12;
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 23;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[5].wrapping_sub(base)) >> (11 - 2);
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 2;
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 13;
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 11 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack11_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 2047);
        if length == 1 { break 'bail; }
        output[1] = base.wrapping_add((tmp >> 11) & 2047);
        if length == 2 { break 'bail; }
        output[2] = tmp >> 22;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[2] |= (tmp % (1u32 << 1)) << (11 - 1);
        output[2] = output[2].wrapping_add(base);
        if length == 3 { break 'bail; }
        output[3] = base.wrapping_add((tmp >> 1) & 2047);
        if length == 4 { break 'bail; }
        output[4] = base.wrapping_add((tmp >> 12) & 2047);
        if length == 5 { break 'bail; }
        output[5] = tmp >> 23;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[5] |= (tmp % (1u32 << 2)) << (11 - 2);
        output[5] = output[5].wrapping_add(base);
        if length == 6 { break 'bail; }
        output[6] = base.wrapping_add((tmp >> 2) & 2047);
        if length == 7 { break 'bail; }
        output[7] = base.wrapping_add((tmp >> 13) & 2047);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 11 + 7) / 8
}

pub fn pack12_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 12;
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 24;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[2].wrapping_sub(base)) >> (12 - 4);
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 4;
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 16;
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 28;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[5].wrapping_sub(base)) >> (12 - 8);
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 8;
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 20;
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 12 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack12_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 4095);
        if length == 1 { break 'bail; }
        output[1] = base.wrapping_add((tmp >> 12) & 4095);
        if length == 2 { break 'bail; }
        output[2] = tmp >> 24;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[2] |= (tmp % (1u32 << 4)) << (12 - 4);
        output[2] = output[2].wrapping_add(base);
        if length == 3 { break 'bail; }
        output[3] = base.wrapping_add((tmp >> 4) & 4095);
        if length == 4 { break 'bail; }
        output[4] = base.wrapping_add((tmp >> 16) & 4095);
        if length == 5 { break 'bail; }
        output[5] = tmp >> 28;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[5] |= (tmp % (1u32 << 8)) << (12 - 8);
        output[5] = output[5].wrapping_add(base);
        if length == 6 { break 'bail; }
        output[6] = base.wrapping_add((tmp >> 8) & 4095);
        if length == 7 { break 'bail; }
        output[7] = base.wrapping_add((tmp >> 20) & 4095);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 12 + 7) / 8
}

pub fn pack13_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 13;
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 26;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[2].wrapping_sub(base)) >> (13 - 7);
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 7;
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 20;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[4].wrapping_sub(base)) >> (13 - 1);
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 1;
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 14;
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 27;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[7].wrapping_sub(base)) >> (13 - 8);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 13 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack13_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 8191);
        if length == 1 { break 'bail; }
        output[1] = base.wrapping_add((tmp >> 13) & 8191);
        if length == 2 { break 'bail; }
        output[2] = tmp >> 26;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[2] |= (tmp % (1u32 << 7)) << (13 - 7);
        output[2] = output[2].wrapping_add(base);
        if length == 3 { break 'bail; }
        output[3] = base.wrapping_add((tmp >> 7) & 8191);
        if length == 4 { break 'bail; }
        output[4] = tmp >> 20;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[4] |= (tmp % (1u32 << 1)) << (13 - 1);
        output[4] = output[4].wrapping_add(base);
        if length == 5 { break 'bail; }
        output[5] = base.wrapping_add((tmp >> 1) & 8191);
        if length == 6 { break 'bail; }
        output[6] = base.wrapping_add((tmp >> 14) & 8191);
        if length == 7 { break 'bail; }
        output[7] = tmp >> 27;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[7] |= (tmp % (1u32 << 8)) << (13 - 8);
        output[7] = output[7].wrapping_add(base);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 13 + 7) / 8
}

pub fn pack14_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 14;
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 28;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[2].wrapping_sub(base)) >> (14 - 10);
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 10;
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 24;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[4].wrapping_sub(base)) >> (14 - 6);
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 6;
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 20;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[6].wrapping_sub(base)) >> (14 - 2);
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 2;
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 14 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack14_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 16383);
        if length == 1 { break 'bail; }
        output[1] = base.wrapping_add((tmp >> 14) & 16383);
        if length == 2 { break 'bail; }
        output[2] = tmp >> 28;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[2] |= (tmp % (1u32 << 10)) << (14 - 10);
        output[2] = output[2].wrapping_add(base);
        if length == 3 { break 'bail; }
        output[3] = base.wrapping_add((tmp >> 10) & 16383);
        if length == 4 { break 'bail; }
        output[4] = tmp >> 24;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[4] |= (tmp % (1u32 << 6)) << (14 - 6);
        output[4] = output[4].wrapping_add(base);
        if length == 5 { break 'bail; }
        output[5] = base.wrapping_add((tmp >> 6) & 16383);
        if length == 6 { break 'bail; }
        output[6] = tmp >> 20;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[6] |= (tmp % (1u32 << 2)) << (14 - 2);
        output[6] = output[6].wrapping_add(base);
        if length == 7 { break 'bail; }
        output[7] = base.wrapping_add((tmp >> 2) & 16383);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 14 + 7) / 8
}

pub fn pack15_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 15;
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 30;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[2].wrapping_sub(base)) >> (15 - 13);
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 13;
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 28;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[4].wrapping_sub(base)) >> (15 - 11);
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 11;
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 26;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[6].wrapping_sub(base)) >> (15 - 9);
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 9;
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 15 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack15_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 32767);
        if length == 1 { break 'bail; }
        output[1] = base.wrapping_add((tmp >> 15) & 32767);
        if length == 2 { break 'bail; }
        output[2] = tmp >> 30;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[2] |= (tmp % (1u32 << 13)) << (15 - 13);
        output[2] = output[2].wrapping_add(base);
        if length == 3 { break 'bail; }
        output[3] = base.wrapping_add((tmp >> 13) & 32767);
        if length == 4 { break 'bail; }
        output[4] = tmp >> 28;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[4] |= (tmp % (1u32 << 11)) << (15 - 11);
        output[4] = output[4].wrapping_add(base);
        if length == 5 { break 'bail; }
        output[5] = base.wrapping_add((tmp >> 11) & 32767);
        if length == 6 { break 'bail; }
        output[6] = tmp >> 26;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[6] |= (tmp % (1u32 << 9)) << (15 - 9);
        output[6] = output[6].wrapping_add(base);
        if length == 7 { break 'bail; }
        output[7] = base.wrapping_add((tmp >> 9) & 32767);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 15 + 7) / 8
}

pub fn pack16_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 16;
        if length == 2 { break 'bail; }
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[2].wrapping_sub(base)) << 0;
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 16;
        if length == 4 { break 'bail; }
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[4].wrapping_sub(base)) << 0;
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 16;
        if length == 6 { break 'bail; }
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[6].wrapping_sub(base)) << 0;
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 16;
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 16 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack16_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 65535);
        if length == 1 { break 'bail; }
        output[1] = base.wrapping_add((tmp >> 16) & 65535);
        if length == 2 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[2] = base.wrapping_add((tmp >> 0) & 65535);
        if length == 3 { break 'bail; }
        output[3] = base.wrapping_add((tmp >> 16) & 65535);
        if length == 4 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[4] = base.wrapping_add((tmp >> 0) & 65535);
        if length == 5 { break 'bail; }
        output[5] = base.wrapping_add((tmp >> 16) & 65535);
        if length == 6 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[6] = base.wrapping_add((tmp >> 0) & 65535);
        if length == 7 { break 'bail; }
        output[7] = base.wrapping_add((tmp >> 16) & 65535);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 16 + 7) / 8
}

pub fn pack17_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 17;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[1].wrapping_sub(base)) >> (17 - 2);
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 2;
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 19;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[3].wrapping_sub(base)) >> (17 - 4);
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 4;
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 21;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[5].wrapping_sub(base)) >> (17 - 6);
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 6;
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 23;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[7].wrapping_sub(base)) >> (17 - 8);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 17 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack17_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 131071);
        if length == 1 { break 'bail; }
        output[1] = tmp >> 17;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[1] |= (tmp % (1u32 << 2)) << (17 - 2);
        output[1] = output[1].wrapping_add(base);
        if length == 2 { break 'bail; }
        output[2] = base.wrapping_add((tmp >> 2) & 131071);
        if length == 3 { break 'bail; }
        output[3] = tmp >> 19;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[3] |= (tmp % (1u32 << 4)) << (17 - 4);
        output[3] = output[3].wrapping_add(base);
        if length == 4 { break 'bail; }
        output[4] = base.wrapping_add((tmp >> 4) & 131071);
        if length == 5 { break 'bail; }
        output[5] = tmp >> 21;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[5] |= (tmp % (1u32 << 6)) << (17 - 6);
        output[5] = output[5].wrapping_add(base);
        if length == 6 { break 'bail; }
        output[6] = base.wrapping_add((tmp >> 6) & 131071);
        if length == 7 { break 'bail; }
        output[7] = tmp >> 23;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[7] |= (tmp % (1u32 << 8)) << (17 - 8);
        output[7] = output[7].wrapping_add(base);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 17 + 7) / 8
}

pub fn pack18_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 18;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[1].wrapping_sub(base)) >> (18 - 4);
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 4;
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 22;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[3].wrapping_sub(base)) >> (18 - 8);
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 8;
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 26;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[5].wrapping_sub(base)) >> (18 - 12);
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 12;
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 30;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[7].wrapping_sub(base)) >> (18 - 16);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 18 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack18_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 262143);
        if length == 1 { break 'bail; }
        output[1] = tmp >> 18;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[1] |= (tmp % (1u32 << 4)) << (18 - 4);
        output[1] = output[1].wrapping_add(base);
        if length == 2 { break 'bail; }
        output[2] = base.wrapping_add((tmp >> 4) & 262143);
        if length == 3 { break 'bail; }
        output[3] = tmp >> 22;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[3] |= (tmp % (1u32 << 8)) << (18 - 8);
        output[3] = output[3].wrapping_add(base);
        if length == 4 { break 'bail; }
        output[4] = base.wrapping_add((tmp >> 8) & 262143);
        if length == 5 { break 'bail; }
        output[5] = tmp >> 26;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[5] |= (tmp % (1u32 << 12)) << (18 - 12);
        output[5] = output[5].wrapping_add(base);
        if length == 6 { break 'bail; }
        output[6] = base.wrapping_add((tmp >> 12) & 262143);
        if length == 7 { break 'bail; }
        output[7] = tmp >> 30;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[7] |= (tmp % (1u32 << 16)) << (18 - 16);
        output[7] = output[7].wrapping_add(base);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 18 + 7) / 8
}

pub fn pack19_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 19;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[1].wrapping_sub(base)) >> (19 - 6);
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 6;
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 25;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[3].wrapping_sub(base)) >> (19 - 12);
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 12;
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 31;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[5].wrapping_sub(base)) >> (19 - 18);
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 18;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[6].wrapping_sub(base)) >> (19 - 5);
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 5;
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 19 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack19_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 524287);
        if length == 1 { break 'bail; }
        output[1] = tmp >> 19;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[1] |= (tmp % (1u32 << 6)) << (19 - 6);
        output[1] = output[1].wrapping_add(base);
        if length == 2 { break 'bail; }
        output[2] = base.wrapping_add((tmp >> 6) & 524287);
        if length == 3 { break 'bail; }
        output[3] = tmp >> 25;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[3] |= (tmp % (1u32 << 12)) << (19 - 12);
        output[3] = output[3].wrapping_add(base);
        if length == 4 { break 'bail; }
        output[4] = base.wrapping_add((tmp >> 12) & 524287);
        if length == 5 { break 'bail; }
        output[5] = tmp >> 31;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[5] |= (tmp % (1u32 << 18)) << (19 - 18);
        output[5] = output[5].wrapping_add(base);
        if length == 6 { break 'bail; }
        output[6] = tmp >> 18;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[6] |= (tmp % (1u32 << 5)) << (19 - 5);
        output[6] = output[6].wrapping_add(base);
        if length == 7 { break 'bail; }
        output[7] = base.wrapping_add((tmp >> 5) & 524287);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 19 + 7) / 8
}

pub fn pack20_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 20;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[1].wrapping_sub(base)) >> (20 - 8);
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 8;
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 28;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[3].wrapping_sub(base)) >> (20 - 16);
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 16;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[4].wrapping_sub(base)) >> (20 - 4);
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 4;
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 24;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[6].wrapping_sub(base)) >> (20 - 12);
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 12;
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 20 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack20_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 1048575);
        if length == 1 { break 'bail; }
        output[1] = tmp >> 20;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[1] |= (tmp % (1u32 << 8)) << (20 - 8);
        output[1] = output[1].wrapping_add(base);
        if length == 2 { break 'bail; }
        output[2] = base.wrapping_add((tmp >> 8) & 1048575);
        if length == 3 { break 'bail; }
        output[3] = tmp >> 28;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[3] |= (tmp % (1u32 << 16)) << (20 - 16);
        output[3] = output[3].wrapping_add(base);
        if length == 4 { break 'bail; }
        output[4] = tmp >> 16;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[4] |= (tmp % (1u32 << 4)) << (20 - 4);
        output[4] = output[4].wrapping_add(base);
        if length == 5 { break 'bail; }
        output[5] = base.wrapping_add((tmp >> 4) & 1048575);
        if length == 6 { break 'bail; }
        output[6] = tmp >> 24;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[6] |= (tmp % (1u32 << 12)) << (20 - 12);
        output[6] = output[6].wrapping_add(base);
        if length == 7 { break 'bail; }
        output[7] = base.wrapping_add((tmp >> 12) & 1048575);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 20 + 7) / 8
}

pub fn pack21_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 21;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[1].wrapping_sub(base)) >> (21 - 10);
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 10;
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 31;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[3].wrapping_sub(base)) >> (21 - 20);
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 20;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[4].wrapping_sub(base)) >> (21 - 9);
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 9;
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 30;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[6].wrapping_sub(base)) >> (21 - 19);
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 19;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[7].wrapping_sub(base)) >> (21 - 8);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 21 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack21_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 2097151);
        if length == 1 { break 'bail; }
        output[1] = tmp >> 21;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[1] |= (tmp % (1u32 << 10)) << (21 - 10);
        output[1] = output[1].wrapping_add(base);
        if length == 2 { break 'bail; }
        output[2] = base.wrapping_add((tmp >> 10) & 2097151);
        if length == 3 { break 'bail; }
        output[3] = tmp >> 31;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[3] |= (tmp % (1u32 << 20)) << (21 - 20);
        output[3] = output[3].wrapping_add(base);
        if length == 4 { break 'bail; }
        output[4] = tmp >> 20;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[4] |= (tmp % (1u32 << 9)) << (21 - 9);
        output[4] = output[4].wrapping_add(base);
        if length == 5 { break 'bail; }
        output[5] = base.wrapping_add((tmp >> 9) & 2097151);
        if length == 6 { break 'bail; }
        output[6] = tmp >> 30;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[6] |= (tmp % (1u32 << 19)) << (21 - 19);
        output[6] = output[6].wrapping_add(base);
        if length == 7 { break 'bail; }
        output[7] = tmp >> 19;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[7] |= (tmp % (1u32 << 8)) << (21 - 8);
        output[7] = output[7].wrapping_add(base);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 21 + 7) / 8
}

pub fn pack22_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 22;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[1].wrapping_sub(base)) >> (22 - 12);
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 12;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[2].wrapping_sub(base)) >> (22 - 2);
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 2;
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 24;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[4].wrapping_sub(base)) >> (22 - 14);
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 14;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[5].wrapping_sub(base)) >> (22 - 4);
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 4;
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 26;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[7].wrapping_sub(base)) >> (22 - 16);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 22 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack22_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 4194303);
        if length == 1 { break 'bail; }
        output[1] = tmp >> 22;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[1] |= (tmp % (1u32 << 12)) << (22 - 12);
        output[1] = output[1].wrapping_add(base);
        if length == 2 { break 'bail; }
        output[2] = tmp >> 12;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[2] |= (tmp % (1u32 << 2)) << (22 - 2);
        output[2] = output[2].wrapping_add(base);
        if length == 3 { break 'bail; }
        output[3] = base.wrapping_add((tmp >> 2) & 4194303);
        if length == 4 { break 'bail; }
        output[4] = tmp >> 24;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[4] |= (tmp % (1u32 << 14)) << (22 - 14);
        output[4] = output[4].wrapping_add(base);
        if length == 5 { break 'bail; }
        output[5] = tmp >> 14;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[5] |= (tmp % (1u32 << 4)) << (22 - 4);
        output[5] = output[5].wrapping_add(base);
        if length == 6 { break 'bail; }
        output[6] = base.wrapping_add((tmp >> 4) & 4194303);
        if length == 7 { break 'bail; }
        output[7] = tmp >> 26;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[7] |= (tmp % (1u32 << 16)) << (22 - 16);
        output[7] = output[7].wrapping_add(base);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 22 + 7) / 8
}

pub fn pack23_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 23;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[1].wrapping_sub(base)) >> (23 - 14);
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 14;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[2].wrapping_sub(base)) >> (23 - 5);
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 5;
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 28;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[4].wrapping_sub(base)) >> (23 - 19);
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 19;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[5].wrapping_sub(base)) >> (23 - 10);
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 10;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[6].wrapping_sub(base)) >> (23 - 1);
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 1;
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 23 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack23_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 8388607);
        if length == 1 { break 'bail; }
        output[1] = tmp >> 23;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[1] |= (tmp % (1u32 << 14)) << (23 - 14);
        output[1] = output[1].wrapping_add(base);
        if length == 2 { break 'bail; }
        output[2] = tmp >> 14;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[2] |= (tmp % (1u32 << 5)) << (23 - 5);
        output[2] = output[2].wrapping_add(base);
        if length == 3 { break 'bail; }
        output[3] = base.wrapping_add((tmp >> 5) & 8388607);
        if length == 4 { break 'bail; }
        output[4] = tmp >> 28;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[4] |= (tmp % (1u32 << 19)) << (23 - 19);
        output[4] = output[4].wrapping_add(base);
        if length == 5 { break 'bail; }
        output[5] = tmp >> 19;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[5] |= (tmp % (1u32 << 10)) << (23 - 10);
        output[5] = output[5].wrapping_add(base);
        if length == 6 { break 'bail; }
        output[6] = tmp >> 10;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[6] |= (tmp % (1u32 << 1)) << (23 - 1);
        output[6] = output[6].wrapping_add(base);
        if length == 7 { break 'bail; }
        output[7] = base.wrapping_add((tmp >> 1) & 8388607);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 23 + 7) / 8
}

pub fn pack24_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 24;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[1].wrapping_sub(base)) >> (24 - 16);
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 16;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[2].wrapping_sub(base)) >> (24 - 8);
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 8;
        if length == 4 { break 'bail; }
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[4].wrapping_sub(base)) << 0;
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 24;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[5].wrapping_sub(base)) >> (24 - 16);
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 16;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[6].wrapping_sub(base)) >> (24 - 8);
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 8;
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 24 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack24_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 16777215);
        if length == 1 { break 'bail; }
        output[1] = tmp >> 24;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[1] |= (tmp % (1u32 << 16)) << (24 - 16);
        output[1] = output[1].wrapping_add(base);
        if length == 2 { break 'bail; }
        output[2] = tmp >> 16;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[2] |= (tmp % (1u32 << 8)) << (24 - 8);
        output[2] = output[2].wrapping_add(base);
        if length == 3 { break 'bail; }
        output[3] = base.wrapping_add((tmp >> 8) & 16777215);
        if length == 4 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[4] = base.wrapping_add((tmp >> 0) & 16777215);
        if length == 5 { break 'bail; }
        output[5] = tmp >> 24;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[5] |= (tmp % (1u32 << 16)) << (24 - 16);
        output[5] = output[5].wrapping_add(base);
        if length == 6 { break 'bail; }
        output[6] = tmp >> 16;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[6] |= (tmp % (1u32 << 8)) << (24 - 8);
        output[6] = output[6].wrapping_add(base);
        if length == 7 { break 'bail; }
        output[7] = base.wrapping_add((tmp >> 8) & 16777215);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 24 + 7) / 8
}

pub fn pack25_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 25;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[1].wrapping_sub(base)) >> (25 - 18);
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 18;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[2].wrapping_sub(base)) >> (25 - 11);
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 11;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[3].wrapping_sub(base)) >> (25 - 4);
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 4;
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 29;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[5].wrapping_sub(base)) >> (25 - 22);
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 22;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[6].wrapping_sub(base)) >> (25 - 15);
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 15;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[7].wrapping_sub(base)) >> (25 - 8);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 25 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack25_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 33554431);
        if length == 1 { break 'bail; }
        output[1] = tmp >> 25;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[1] |= (tmp % (1u32 << 18)) << (25 - 18);
        output[1] = output[1].wrapping_add(base);
        if length == 2 { break 'bail; }
        output[2] = tmp >> 18;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[2] |= (tmp % (1u32 << 11)) << (25 - 11);
        output[2] = output[2].wrapping_add(base);
        if length == 3 { break 'bail; }
        output[3] = tmp >> 11;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[3] |= (tmp % (1u32 << 4)) << (25 - 4);
        output[3] = output[3].wrapping_add(base);
        if length == 4 { break 'bail; }
        output[4] = base.wrapping_add((tmp >> 4) & 33554431);
        if length == 5 { break 'bail; }
        output[5] = tmp >> 29;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[5] |= (tmp % (1u32 << 22)) << (25 - 22);
        output[5] = output[5].wrapping_add(base);
        if length == 6 { break 'bail; }
        output[6] = tmp >> 22;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[6] |= (tmp % (1u32 << 15)) << (25 - 15);
        output[6] = output[6].wrapping_add(base);
        if length == 7 { break 'bail; }
        output[7] = tmp >> 15;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[7] |= (tmp % (1u32 << 8)) << (25 - 8);
        output[7] = output[7].wrapping_add(base);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 25 + 7) / 8
}

pub fn pack26_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 26;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[1].wrapping_sub(base)) >> (26 - 20);
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 20;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[2].wrapping_sub(base)) >> (26 - 14);
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 14;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[3].wrapping_sub(base)) >> (26 - 8);
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 8;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[4].wrapping_sub(base)) >> (26 - 2);
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 2;
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 28;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[6].wrapping_sub(base)) >> (26 - 22);
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 22;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[7].wrapping_sub(base)) >> (26 - 16);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 26 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack26_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 67108863);
        if length == 1 { break 'bail; }
        output[1] = tmp >> 26;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[1] |= (tmp % (1u32 << 20)) << (26 - 20);
        output[1] = output[1].wrapping_add(base);
        if length == 2 { break 'bail; }
        output[2] = tmp >> 20;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[2] |= (tmp % (1u32 << 14)) << (26 - 14);
        output[2] = output[2].wrapping_add(base);
        if length == 3 { break 'bail; }
        output[3] = tmp >> 14;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[3] |= (tmp % (1u32 << 8)) << (26 - 8);
        output[3] = output[3].wrapping_add(base);
        if length == 4 { break 'bail; }
        output[4] = tmp >> 8;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[4] |= (tmp % (1u32 << 2)) << (26 - 2);
        output[4] = output[4].wrapping_add(base);
        if length == 5 { break 'bail; }
        output[5] = base.wrapping_add((tmp >> 2) & 67108863);
        if length == 6 { break 'bail; }
        output[6] = tmp >> 28;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[6] |= (tmp % (1u32 << 22)) << (26 - 22);
        output[6] = output[6].wrapping_add(base);
        if length == 7 { break 'bail; }
        output[7] = tmp >> 22;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[7] |= (tmp % (1u32 << 16)) << (26 - 16);
        output[7] = output[7].wrapping_add(base);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 26 + 7) / 8
}

pub fn pack27_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 27;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[1].wrapping_sub(base)) >> (27 - 22);
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 22;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[2].wrapping_sub(base)) >> (27 - 17);
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 17;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[3].wrapping_sub(base)) >> (27 - 12);
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 12;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[4].wrapping_sub(base)) >> (27 - 7);
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 7;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[5].wrapping_sub(base)) >> (27 - 2);
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 2;
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 29;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[7].wrapping_sub(base)) >> (27 - 24);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 27 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack27_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 134217727);
        if length == 1 { break 'bail; }
        output[1] = tmp >> 27;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[1] |= (tmp % (1u32 << 22)) << (27 - 22);
        output[1] = output[1].wrapping_add(base);
        if length == 2 { break 'bail; }
        output[2] = tmp >> 22;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[2] |= (tmp % (1u32 << 17)) << (27 - 17);
        output[2] = output[2].wrapping_add(base);
        if length == 3 { break 'bail; }
        output[3] = tmp >> 17;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[3] |= (tmp % (1u32 << 12)) << (27 - 12);
        output[3] = output[3].wrapping_add(base);
        if length == 4 { break 'bail; }
        output[4] = tmp >> 12;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[4] |= (tmp % (1u32 << 7)) << (27 - 7);
        output[4] = output[4].wrapping_add(base);
        if length == 5 { break 'bail; }
        output[5] = tmp >> 7;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[5] |= (tmp % (1u32 << 2)) << (27 - 2);
        output[5] = output[5].wrapping_add(base);
        if length == 6 { break 'bail; }
        output[6] = base.wrapping_add((tmp >> 2) & 134217727);
        if length == 7 { break 'bail; }
        output[7] = tmp >> 29;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[7] |= (tmp % (1u32 << 24)) << (27 - 24);
        output[7] = output[7].wrapping_add(base);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 27 + 7) / 8
}

pub fn pack28_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 28;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[1].wrapping_sub(base)) >> (28 - 24);
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 24;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[2].wrapping_sub(base)) >> (28 - 20);
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 20;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[3].wrapping_sub(base)) >> (28 - 16);
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 16;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[4].wrapping_sub(base)) >> (28 - 12);
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 12;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[5].wrapping_sub(base)) >> (28 - 8);
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 8;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[6].wrapping_sub(base)) >> (28 - 4);
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 4;
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 28 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack28_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 268435455);
        if length == 1 { break 'bail; }
        output[1] = tmp >> 28;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[1] |= (tmp % (1u32 << 24)) << (28 - 24);
        output[1] = output[1].wrapping_add(base);
        if length == 2 { break 'bail; }
        output[2] = tmp >> 24;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[2] |= (tmp % (1u32 << 20)) << (28 - 20);
        output[2] = output[2].wrapping_add(base);
        if length == 3 { break 'bail; }
        output[3] = tmp >> 20;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[3] |= (tmp % (1u32 << 16)) << (28 - 16);
        output[3] = output[3].wrapping_add(base);
        if length == 4 { break 'bail; }
        output[4] = tmp >> 16;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[4] |= (tmp % (1u32 << 12)) << (28 - 12);
        output[4] = output[4].wrapping_add(base);
        if length == 5 { break 'bail; }
        output[5] = tmp >> 12;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[5] |= (tmp % (1u32 << 8)) << (28 - 8);
        output[5] = output[5].wrapping_add(base);
        if length == 6 { break 'bail; }
        output[6] = tmp >> 8;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[6] |= (tmp % (1u32 << 4)) << (28 - 4);
        output[6] = output[6].wrapping_add(base);
        if length == 7 { break 'bail; }
        output[7] = base.wrapping_add((tmp >> 4) & 268435455);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 28 + 7) / 8
}

pub fn pack29_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 29;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[1].wrapping_sub(base)) >> (29 - 26);
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 26;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[2].wrapping_sub(base)) >> (29 - 23);
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 23;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[3].wrapping_sub(base)) >> (29 - 20);
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 20;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[4].wrapping_sub(base)) >> (29 - 17);
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 17;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[5].wrapping_sub(base)) >> (29 - 14);
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 14;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[6].wrapping_sub(base)) >> (29 - 11);
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 11;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[7].wrapping_sub(base)) >> (29 - 8);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 29 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack29_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 536870911);
        if length == 1 { break 'bail; }
        output[1] = tmp >> 29;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[1] |= (tmp % (1u32 << 26)) << (29 - 26);
        output[1] = output[1].wrapping_add(base);
        if length == 2 { break 'bail; }
        output[2] = tmp >> 26;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[2] |= (tmp % (1u32 << 23)) << (29 - 23);
        output[2] = output[2].wrapping_add(base);
        if length == 3 { break 'bail; }
        output[3] = tmp >> 23;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[3] |= (tmp % (1u32 << 20)) << (29 - 20);
        output[3] = output[3].wrapping_add(base);
        if length == 4 { break 'bail; }
        output[4] = tmp >> 20;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[4] |= (tmp % (1u32 << 17)) << (29 - 17);
        output[4] = output[4].wrapping_add(base);
        if length == 5 { break 'bail; }
        output[5] = tmp >> 17;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[5] |= (tmp % (1u32 << 14)) << (29 - 14);
        output[5] = output[5].wrapping_add(base);
        if length == 6 { break 'bail; }
        output[6] = tmp >> 14;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[6] |= (tmp % (1u32 << 11)) << (29 - 11);
        output[6] = output[6].wrapping_add(base);
        if length == 7 { break 'bail; }
        output[7] = tmp >> 11;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[7] |= (tmp % (1u32 << 8)) << (29 - 8);
        output[7] = output[7].wrapping_add(base);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 29 + 7) / 8
}

pub fn pack30_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 30;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[1].wrapping_sub(base)) >> (30 - 28);
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 28;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[2].wrapping_sub(base)) >> (30 - 26);
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 26;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[3].wrapping_sub(base)) >> (30 - 24);
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 24;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[4].wrapping_sub(base)) >> (30 - 22);
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 22;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[5].wrapping_sub(base)) >> (30 - 20);
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 20;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[6].wrapping_sub(base)) >> (30 - 18);
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 18;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[7].wrapping_sub(base)) >> (30 - 16);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 30 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack30_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 1073741823);
        if length == 1 { break 'bail; }
        output[1] = tmp >> 30;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[1] |= (tmp % (1u32 << 28)) << (30 - 28);
        output[1] = output[1].wrapping_add(base);
        if length == 2 { break 'bail; }
        output[2] = tmp >> 28;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[2] |= (tmp % (1u32 << 26)) << (30 - 26);
        output[2] = output[2].wrapping_add(base);
        if length == 3 { break 'bail; }
        output[3] = tmp >> 26;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[3] |= (tmp % (1u32 << 24)) << (30 - 24);
        output[3] = output[3].wrapping_add(base);
        if length == 4 { break 'bail; }
        output[4] = tmp >> 24;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[4] |= (tmp % (1u32 << 22)) << (30 - 22);
        output[4] = output[4].wrapping_add(base);
        if length == 5 { break 'bail; }
        output[5] = tmp >> 22;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[5] |= (tmp % (1u32 << 20)) << (30 - 20);
        output[5] = output[5].wrapping_add(base);
        if length == 6 { break 'bail; }
        output[6] = tmp >> 20;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[6] |= (tmp % (1u32 << 18)) << (30 - 18);
        output[6] = output[6].wrapping_add(base);
        if length == 7 { break 'bail; }
        output[7] = tmp >> 18;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[7] |= (tmp % (1u32 << 16)) << (30 - 16);
        output[7] = output[7].wrapping_add(base);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 30 + 7) / 8
}

pub fn pack31_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        tmp |= (input[1].wrapping_sub(base)) << 31;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[1].wrapping_sub(base)) >> (31 - 30);
        if length == 2 { break 'bail; }
        tmp |= (input[2].wrapping_sub(base)) << 30;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[2].wrapping_sub(base)) >> (31 - 29);
        if length == 3 { break 'bail; }
        tmp |= (input[3].wrapping_sub(base)) << 29;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[3].wrapping_sub(base)) >> (31 - 28);
        if length == 4 { break 'bail; }
        tmp |= (input[4].wrapping_sub(base)) << 28;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[4].wrapping_sub(base)) >> (31 - 27);
        if length == 5 { break 'bail; }
        tmp |= (input[5].wrapping_sub(base)) << 27;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[5].wrapping_sub(base)) >> (31 - 26);
        if length == 6 { break 'bail; }
        tmp |= (input[6].wrapping_sub(base)) << 26;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[6].wrapping_sub(base)) >> (31 - 25);
        if length == 7 { break 'bail; }
        tmp |= (input[7].wrapping_sub(base)) << 25;
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[7].wrapping_sub(base)) >> (31 - 24);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 31 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack31_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 2147483647);
        if length == 1 { break 'bail; }
        output[1] = tmp >> 31;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[1] |= (tmp % (1u32 << 30)) << (31 - 30);
        output[1] = output[1].wrapping_add(base);
        if length == 2 { break 'bail; }
        output[2] = tmp >> 30;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[2] |= (tmp % (1u32 << 29)) << (31 - 29);
        output[2] = output[2].wrapping_add(base);
        if length == 3 { break 'bail; }
        output[3] = tmp >> 29;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[3] |= (tmp % (1u32 << 28)) << (31 - 28);
        output[3] = output[3].wrapping_add(base);
        if length == 4 { break 'bail; }
        output[4] = tmp >> 28;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[4] |= (tmp % (1u32 << 27)) << (31 - 27);
        output[4] = output[4].wrapping_add(base);
        if length == 5 { break 'bail; }
        output[5] = tmp >> 27;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[5] |= (tmp % (1u32 << 26)) << (31 - 26);
        output[5] = output[5].wrapping_add(base);
        if length == 6 { break 'bail; }
        output[6] = tmp >> 26;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[6] |= (tmp % (1u32 << 25)) << (31 - 25);
        output[6] = output[6].wrapping_add(base);
        if length == 7 { break 'bail; }
        output[7] = tmp >> 25;
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[7] |= (tmp % (1u32 << 24)) << (31 - 24);
        output[7] = output[7].wrapping_add(base);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 31 + 7) / 8
}

pub fn pack32_x(base: u32, input: &[u32], output: &mut [u8], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut out_off: usize = 0;
    'bail: loop {
        tmp = (input[0].wrapping_sub(base)) << 0;
        if length == 1 { break 'bail; }
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[1].wrapping_sub(base)) << 0;
        if length == 2 { break 'bail; }
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[2].wrapping_sub(base)) << 0;
        if length == 3 { break 'bail; }
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[3].wrapping_sub(base)) << 0;
        if length == 4 { break 'bail; }
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[4].wrapping_sub(base)) << 0;
        if length == 5 { break 'bail; }
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[5].wrapping_sub(base)) << 0;
        if length == 6 { break 'bail; }
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[6].wrapping_sub(base)) << 0;
        if length == 7 { break 'bail; }
        write_u32(output, out_off, tmp);
        out_off += 4;
        tmp = (input[7].wrapping_sub(base)) << 0;
        if length == 8 { break 'bail; }
        break 'bail;
    }
    let total: u32 = (length * 32 + 7) / 8;
    let mut remaining: usize = (total as usize) % 4;
    if remaining == 0 { remaining = 4; }
    write_partial(output, out_off, tmp, remaining);
    total
}

pub fn unpack32_x(base: u32, input: &[u8], output: &mut [u32], length: u32) -> u32 {
    if length == 0 { return 0; }
    let mut tmp: u32 = 0;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        output[0] = base.wrapping_add((tmp >> 0) & 4294967295);
        if length == 1 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[1] = base.wrapping_add((tmp >> 0) & 4294967295);
        if length == 2 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[2] = base.wrapping_add((tmp >> 0) & 4294967295);
        if length == 3 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[3] = base.wrapping_add((tmp >> 0) & 4294967295);
        if length == 4 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[4] = base.wrapping_add((tmp >> 0) & 4294967295);
        if length == 5 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[5] = base.wrapping_add((tmp >> 0) & 4294967295);
        if length == 6 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[6] = base.wrapping_add((tmp >> 0) & 4294967295);
        if length == 7 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        output[7] = base.wrapping_add((tmp >> 0) & 4294967295);
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 32 + 7) / 8
}

pub fn linsearch1_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 1) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 1) & 1) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 2) & 1) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 3) & 1) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 4) & 1) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 5) & 1) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 6) & 1) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 7) & 1) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 8) & 1) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 9) & 1) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 10) & 1) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 11) & 1) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 12) & 1) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 13) & 1) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 14) & 1) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 15) & 1) == value {
        *found = 15;
        return 15;
    }
    if ((tmp >> 16) & 1) == value {
        *found = 16;
        return 16;
    }
    if ((tmp >> 17) & 1) == value {
        *found = 17;
        return 17;
    }
    if ((tmp >> 18) & 1) == value {
        *found = 18;
        return 18;
    }
    if ((tmp >> 19) & 1) == value {
        *found = 19;
        return 19;
    }
    if ((tmp >> 20) & 1) == value {
        *found = 20;
        return 20;
    }
    if ((tmp >> 21) & 1) == value {
        *found = 21;
        return 21;
    }
    if ((tmp >> 22) & 1) == value {
        *found = 22;
        return 22;
    }
    if ((tmp >> 23) & 1) == value {
        *found = 23;
        return 23;
    }
    if ((tmp >> 24) & 1) == value {
        *found = 24;
        return 24;
    }
    if ((tmp >> 25) & 1) == value {
        *found = 25;
        return 25;
    }
    if ((tmp >> 26) & 1) == value {
        *found = 26;
        return 26;
    }
    if ((tmp >> 27) & 1) == value {
        *found = 27;
        return 27;
    }
    if ((tmp >> 28) & 1) == value {
        *found = 28;
        return 28;
    }
    if ((tmp >> 29) & 1) == value {
        *found = 29;
        return 29;
    }
    if ((tmp >> 30) & 1) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 31) & 1) == value {
        *found = 31;
        return 31;
    }
    4
}

pub fn linsearch2_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 3) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 2) & 3) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 4) & 3) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 6) & 3) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 8) & 3) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 10) & 3) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 12) & 3) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 14) & 3) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 16) & 3) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 18) & 3) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 20) & 3) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 22) & 3) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 24) & 3) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 26) & 3) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 28) & 3) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 30) & 3) == value {
        *found = 15;
        return 15;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 3) == value {
        *found = 16;
        return 16;
    }
    if ((tmp >> 2) & 3) == value {
        *found = 17;
        return 17;
    }
    if ((tmp >> 4) & 3) == value {
        *found = 18;
        return 18;
    }
    if ((tmp >> 6) & 3) == value {
        *found = 19;
        return 19;
    }
    if ((tmp >> 8) & 3) == value {
        *found = 20;
        return 20;
    }
    if ((tmp >> 10) & 3) == value {
        *found = 21;
        return 21;
    }
    if ((tmp >> 12) & 3) == value {
        *found = 22;
        return 22;
    }
    if ((tmp >> 14) & 3) == value {
        *found = 23;
        return 23;
    }
    if ((tmp >> 16) & 3) == value {
        *found = 24;
        return 24;
    }
    if ((tmp >> 18) & 3) == value {
        *found = 25;
        return 25;
    }
    if ((tmp >> 20) & 3) == value {
        *found = 26;
        return 26;
    }
    if ((tmp >> 22) & 3) == value {
        *found = 27;
        return 27;
    }
    if ((tmp >> 24) & 3) == value {
        *found = 28;
        return 28;
    }
    if ((tmp >> 26) & 3) == value {
        *found = 29;
        return 29;
    }
    if ((tmp >> 28) & 3) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 30) & 3) == value {
        *found = 31;
        return 31;
    }
    8
}

pub fn linsearch3_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 7) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 3) & 7) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 6) & 7) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 9) & 7) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 12) & 7) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 15) & 7) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 18) & 7) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 21) & 7) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 24) & 7) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 27) & 7) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (3 - 1)) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 1) & 7) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 4) & 7) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 7) & 7) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 10) & 7) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 13) & 7) == value {
        *found = 15;
        return 15;
    }
    if ((tmp >> 16) & 7) == value {
        *found = 16;
        return 16;
    }
    if ((tmp >> 19) & 7) == value {
        *found = 17;
        return 17;
    }
    if ((tmp >> 22) & 7) == value {
        *found = 18;
        return 18;
    }
    if ((tmp >> 25) & 7) == value {
        *found = 19;
        return 19;
    }
    if ((tmp >> 28) & 7) == value {
        *found = 20;
        return 20;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (3 - 2)) == value {
        *found = 21;
        return 21;
    }
    if ((tmp >> 2) & 7) == value {
        *found = 22;
        return 22;
    }
    if ((tmp >> 5) & 7) == value {
        *found = 23;
        return 23;
    }
    if ((tmp >> 8) & 7) == value {
        *found = 24;
        return 24;
    }
    if ((tmp >> 11) & 7) == value {
        *found = 25;
        return 25;
    }
    if ((tmp >> 14) & 7) == value {
        *found = 26;
        return 26;
    }
    if ((tmp >> 17) & 7) == value {
        *found = 27;
        return 27;
    }
    if ((tmp >> 20) & 7) == value {
        *found = 28;
        return 28;
    }
    if ((tmp >> 23) & 7) == value {
        *found = 29;
        return 29;
    }
    if ((tmp >> 26) & 7) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 29) & 7) == value {
        *found = 31;
        return 31;
    }
    12
}

pub fn linsearch4_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 15) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 4) & 15) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 8) & 15) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 12) & 15) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 16) & 15) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 20) & 15) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 24) & 15) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 28) & 15) == value {
        *found = 7;
        return 7;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 15) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 4) & 15) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 8) & 15) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 12) & 15) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 16) & 15) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 20) & 15) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 24) & 15) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 28) & 15) == value {
        *found = 15;
        return 15;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 15) == value {
        *found = 16;
        return 16;
    }
    if ((tmp >> 4) & 15) == value {
        *found = 17;
        return 17;
    }
    if ((tmp >> 8) & 15) == value {
        *found = 18;
        return 18;
    }
    if ((tmp >> 12) & 15) == value {
        *found = 19;
        return 19;
    }
    if ((tmp >> 16) & 15) == value {
        *found = 20;
        return 20;
    }
    if ((tmp >> 20) & 15) == value {
        *found = 21;
        return 21;
    }
    if ((tmp >> 24) & 15) == value {
        *found = 22;
        return 22;
    }
    if ((tmp >> 28) & 15) == value {
        *found = 23;
        return 23;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 15) == value {
        *found = 24;
        return 24;
    }
    if ((tmp >> 4) & 15) == value {
        *found = 25;
        return 25;
    }
    if ((tmp >> 8) & 15) == value {
        *found = 26;
        return 26;
    }
    if ((tmp >> 12) & 15) == value {
        *found = 27;
        return 27;
    }
    if ((tmp >> 16) & 15) == value {
        *found = 28;
        return 28;
    }
    if ((tmp >> 20) & 15) == value {
        *found = 29;
        return 29;
    }
    if ((tmp >> 24) & 15) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 28) & 15) == value {
        *found = 31;
        return 31;
    }
    16
}

pub fn linsearch5_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 31) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 5) & 31) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 10) & 31) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 15) & 31) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 20) & 31) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 25) & 31) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 3)) << (5 - 3)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 3) & 31) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 8) & 31) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 13) & 31) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 18) & 31) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 23) & 31) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (5 - 1)) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 1) & 31) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 6) & 31) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 11) & 31) == value {
        *found = 15;
        return 15;
    }
    if ((tmp >> 16) & 31) == value {
        *found = 16;
        return 16;
    }
    if ((tmp >> 21) & 31) == value {
        *found = 17;
        return 17;
    }
    if ((tmp >> 26) & 31) == value {
        *found = 18;
        return 18;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (5 - 4)) == value {
        *found = 19;
        return 19;
    }
    if ((tmp >> 4) & 31) == value {
        *found = 20;
        return 20;
    }
    if ((tmp >> 9) & 31) == value {
        *found = 21;
        return 21;
    }
    if ((tmp >> 14) & 31) == value {
        *found = 22;
        return 22;
    }
    if ((tmp >> 19) & 31) == value {
        *found = 23;
        return 23;
    }
    if ((tmp >> 24) & 31) == value {
        *found = 24;
        return 24;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (5 - 2)) == value {
        *found = 25;
        return 25;
    }
    if ((tmp >> 2) & 31) == value {
        *found = 26;
        return 26;
    }
    if ((tmp >> 7) & 31) == value {
        *found = 27;
        return 27;
    }
    if ((tmp >> 12) & 31) == value {
        *found = 28;
        return 28;
    }
    if ((tmp >> 17) & 31) == value {
        *found = 29;
        return 29;
    }
    if ((tmp >> 22) & 31) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 27) & 31) == value {
        *found = 31;
        return 31;
    }
    20
}

pub fn linsearch6_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 63) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 6) & 63) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 12) & 63) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 18) & 63) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 24) & 63) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (6 - 4)) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 4) & 63) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 10) & 63) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 16) & 63) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 22) & 63) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (6 - 2)) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 2) & 63) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 8) & 63) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 14) & 63) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 20) & 63) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 26) & 63) == value {
        *found = 15;
        return 15;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 63) == value {
        *found = 16;
        return 16;
    }
    if ((tmp >> 6) & 63) == value {
        *found = 17;
        return 17;
    }
    if ((tmp >> 12) & 63) == value {
        *found = 18;
        return 18;
    }
    if ((tmp >> 18) & 63) == value {
        *found = 19;
        return 19;
    }
    if ((tmp >> 24) & 63) == value {
        *found = 20;
        return 20;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (6 - 4)) == value {
        *found = 21;
        return 21;
    }
    if ((tmp >> 4) & 63) == value {
        *found = 22;
        return 22;
    }
    if ((tmp >> 10) & 63) == value {
        *found = 23;
        return 23;
    }
    if ((tmp >> 16) & 63) == value {
        *found = 24;
        return 24;
    }
    if ((tmp >> 22) & 63) == value {
        *found = 25;
        return 25;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (6 - 2)) == value {
        *found = 26;
        return 26;
    }
    if ((tmp >> 2) & 63) == value {
        *found = 27;
        return 27;
    }
    if ((tmp >> 8) & 63) == value {
        *found = 28;
        return 28;
    }
    if ((tmp >> 14) & 63) == value {
        *found = 29;
        return 29;
    }
    if ((tmp >> 20) & 63) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 26) & 63) == value {
        *found = 31;
        return 31;
    }
    24
}

pub fn linsearch7_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 127) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 7) & 127) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 14) & 127) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 21) & 127) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 3)) << (7 - 3)) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 3) & 127) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 10) & 127) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 17) & 127) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 24) & 127) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (7 - 6)) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 6) & 127) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 13) & 127) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 20) & 127) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (7 - 2)) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 2) & 127) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 9) & 127) == value {
        *found = 15;
        return 15;
    }
    if ((tmp >> 16) & 127) == value {
        *found = 16;
        return 16;
    }
    if ((tmp >> 23) & 127) == value {
        *found = 17;
        return 17;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 5)) << (7 - 5)) == value {
        *found = 18;
        return 18;
    }
    if ((tmp >> 5) & 127) == value {
        *found = 19;
        return 19;
    }
    if ((tmp >> 12) & 127) == value {
        *found = 20;
        return 20;
    }
    if ((tmp >> 19) & 127) == value {
        *found = 21;
        return 21;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (7 - 1)) == value {
        *found = 22;
        return 22;
    }
    if ((tmp >> 1) & 127) == value {
        *found = 23;
        return 23;
    }
    if ((tmp >> 8) & 127) == value {
        *found = 24;
        return 24;
    }
    if ((tmp >> 15) & 127) == value {
        *found = 25;
        return 25;
    }
    if ((tmp >> 22) & 127) == value {
        *found = 26;
        return 26;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (7 - 4)) == value {
        *found = 27;
        return 27;
    }
    if ((tmp >> 4) & 127) == value {
        *found = 28;
        return 28;
    }
    if ((tmp >> 11) & 127) == value {
        *found = 29;
        return 29;
    }
    if ((tmp >> 18) & 127) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 25) & 127) == value {
        *found = 31;
        return 31;
    }
    28
}

pub fn linsearch8_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 255) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 8) & 255) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 16) & 255) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 24) & 255) == value {
        *found = 3;
        return 3;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 255) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 8) & 255) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 16) & 255) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 24) & 255) == value {
        *found = 7;
        return 7;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 255) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 8) & 255) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 16) & 255) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 24) & 255) == value {
        *found = 11;
        return 11;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 255) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 8) & 255) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 16) & 255) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 24) & 255) == value {
        *found = 15;
        return 15;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 255) == value {
        *found = 16;
        return 16;
    }
    if ((tmp >> 8) & 255) == value {
        *found = 17;
        return 17;
    }
    if ((tmp >> 16) & 255) == value {
        *found = 18;
        return 18;
    }
    if ((tmp >> 24) & 255) == value {
        *found = 19;
        return 19;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 255) == value {
        *found = 20;
        return 20;
    }
    if ((tmp >> 8) & 255) == value {
        *found = 21;
        return 21;
    }
    if ((tmp >> 16) & 255) == value {
        *found = 22;
        return 22;
    }
    if ((tmp >> 24) & 255) == value {
        *found = 23;
        return 23;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 255) == value {
        *found = 24;
        return 24;
    }
    if ((tmp >> 8) & 255) == value {
        *found = 25;
        return 25;
    }
    if ((tmp >> 16) & 255) == value {
        *found = 26;
        return 26;
    }
    if ((tmp >> 24) & 255) == value {
        *found = 27;
        return 27;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 255) == value {
        *found = 28;
        return 28;
    }
    if ((tmp >> 8) & 255) == value {
        *found = 29;
        return 29;
    }
    if ((tmp >> 16) & 255) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 24) & 255) == value {
        *found = 31;
        return 31;
    }
    32
}

pub fn linsearch9_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 511) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 9) & 511) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 18) & 511) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (9 - 4)) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 4) & 511) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 13) & 511) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 22) & 511) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (9 - 8)) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 8) & 511) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 17) & 511) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 3)) << (9 - 3)) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 3) & 511) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 12) & 511) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 21) & 511) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 7)) << (9 - 7)) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 7) & 511) == value {
        *found = 15;
        return 15;
    }
    if ((tmp >> 16) & 511) == value {
        *found = 16;
        return 16;
    }
    tmp2 = tmp >> 25;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (9 - 2)) == value {
        *found = 17;
        return 17;
    }
    if ((tmp >> 2) & 511) == value {
        *found = 18;
        return 18;
    }
    if ((tmp >> 11) & 511) == value {
        *found = 19;
        return 19;
    }
    if ((tmp >> 20) & 511) == value {
        *found = 20;
        return 20;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (9 - 6)) == value {
        *found = 21;
        return 21;
    }
    if ((tmp >> 6) & 511) == value {
        *found = 22;
        return 22;
    }
    if ((tmp >> 15) & 511) == value {
        *found = 23;
        return 23;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (9 - 1)) == value {
        *found = 24;
        return 24;
    }
    if ((tmp >> 1) & 511) == value {
        *found = 25;
        return 25;
    }
    if ((tmp >> 10) & 511) == value {
        *found = 26;
        return 26;
    }
    if ((tmp >> 19) & 511) == value {
        *found = 27;
        return 27;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 5)) << (9 - 5)) == value {
        *found = 28;
        return 28;
    }
    if ((tmp >> 5) & 511) == value {
        *found = 29;
        return 29;
    }
    if ((tmp >> 14) & 511) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 23) & 511) == value {
        *found = 31;
        return 31;
    }
    36
}

pub fn linsearch10_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 1023) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 10) & 1023) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 20) & 1023) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (10 - 8)) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 8) & 1023) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 18) & 1023) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (10 - 6)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 6) & 1023) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 16) & 1023) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (10 - 4)) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 4) & 1023) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 14) & 1023) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (10 - 2)) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 2) & 1023) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 12) & 1023) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 22) & 1023) == value {
        *found = 15;
        return 15;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 1023) == value {
        *found = 16;
        return 16;
    }
    if ((tmp >> 10) & 1023) == value {
        *found = 17;
        return 17;
    }
    if ((tmp >> 20) & 1023) == value {
        *found = 18;
        return 18;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (10 - 8)) == value {
        *found = 19;
        return 19;
    }
    if ((tmp >> 8) & 1023) == value {
        *found = 20;
        return 20;
    }
    if ((tmp >> 18) & 1023) == value {
        *found = 21;
        return 21;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (10 - 6)) == value {
        *found = 22;
        return 22;
    }
    if ((tmp >> 6) & 1023) == value {
        *found = 23;
        return 23;
    }
    if ((tmp >> 16) & 1023) == value {
        *found = 24;
        return 24;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (10 - 4)) == value {
        *found = 25;
        return 25;
    }
    if ((tmp >> 4) & 1023) == value {
        *found = 26;
        return 26;
    }
    if ((tmp >> 14) & 1023) == value {
        *found = 27;
        return 27;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (10 - 2)) == value {
        *found = 28;
        return 28;
    }
    if ((tmp >> 2) & 1023) == value {
        *found = 29;
        return 29;
    }
    if ((tmp >> 12) & 1023) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 22) & 1023) == value {
        *found = 31;
        return 31;
    }
    40
}

pub fn linsearch11_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 2047) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 11) & 2047) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (11 - 1)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 1) & 2047) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 12) & 2047) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 23;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (11 - 2)) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 2) & 2047) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 13) & 2047) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 3)) << (11 - 3)) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 3) & 2047) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 14) & 2047) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 25;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (11 - 4)) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 4) & 2047) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 15) & 2047) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 5)) << (11 - 5)) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 5) & 2047) == value {
        *found = 15;
        return 15;
    }
    if ((tmp >> 16) & 2047) == value {
        *found = 16;
        return 16;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (11 - 6)) == value {
        *found = 17;
        return 17;
    }
    if ((tmp >> 6) & 2047) == value {
        *found = 18;
        return 18;
    }
    if ((tmp >> 17) & 2047) == value {
        *found = 19;
        return 19;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 7)) << (11 - 7)) == value {
        *found = 20;
        return 20;
    }
    if ((tmp >> 7) & 2047) == value {
        *found = 21;
        return 21;
    }
    if ((tmp >> 18) & 2047) == value {
        *found = 22;
        return 22;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (11 - 8)) == value {
        *found = 23;
        return 23;
    }
    if ((tmp >> 8) & 2047) == value {
        *found = 24;
        return 24;
    }
    if ((tmp >> 19) & 2047) == value {
        *found = 25;
        return 25;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 9)) << (11 - 9)) == value {
        *found = 26;
        return 26;
    }
    if ((tmp >> 9) & 2047) == value {
        *found = 27;
        return 27;
    }
    if ((tmp >> 20) & 2047) == value {
        *found = 28;
        return 28;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (11 - 10)) == value {
        *found = 29;
        return 29;
    }
    if ((tmp >> 10) & 2047) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 21) & 2047) == value {
        *found = 31;
        return 31;
    }
    44
}

pub fn linsearch12_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 4095) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 12) & 4095) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (12 - 4)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 4) & 4095) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 16) & 4095) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (12 - 8)) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 8) & 4095) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 20) & 4095) == value {
        *found = 7;
        return 7;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 4095) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 12) & 4095) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (12 - 4)) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 4) & 4095) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 16) & 4095) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (12 - 8)) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 8) & 4095) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 20) & 4095) == value {
        *found = 15;
        return 15;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 4095) == value {
        *found = 16;
        return 16;
    }
    if ((tmp >> 12) & 4095) == value {
        *found = 17;
        return 17;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (12 - 4)) == value {
        *found = 18;
        return 18;
    }
    if ((tmp >> 4) & 4095) == value {
        *found = 19;
        return 19;
    }
    if ((tmp >> 16) & 4095) == value {
        *found = 20;
        return 20;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (12 - 8)) == value {
        *found = 21;
        return 21;
    }
    if ((tmp >> 8) & 4095) == value {
        *found = 22;
        return 22;
    }
    if ((tmp >> 20) & 4095) == value {
        *found = 23;
        return 23;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 4095) == value {
        *found = 24;
        return 24;
    }
    if ((tmp >> 12) & 4095) == value {
        *found = 25;
        return 25;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (12 - 4)) == value {
        *found = 26;
        return 26;
    }
    if ((tmp >> 4) & 4095) == value {
        *found = 27;
        return 27;
    }
    if ((tmp >> 16) & 4095) == value {
        *found = 28;
        return 28;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (12 - 8)) == value {
        *found = 29;
        return 29;
    }
    if ((tmp >> 8) & 4095) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 20) & 4095) == value {
        *found = 31;
        return 31;
    }
    48
}

pub fn linsearch13_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 8191) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 13) & 8191) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 7)) << (13 - 7)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 7) & 8191) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (13 - 1)) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 1) & 8191) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 14) & 8191) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (13 - 8)) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 8) & 8191) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 21;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (13 - 2)) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 2) & 8191) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 15) & 8191) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 9)) << (13 - 9)) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 9) & 8191) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 3)) << (13 - 3)) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 3) & 8191) == value {
        *found = 15;
        return 15;
    }
    if ((tmp >> 16) & 8191) == value {
        *found = 16;
        return 16;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (13 - 10)) == value {
        *found = 17;
        return 17;
    }
    if ((tmp >> 10) & 8191) == value {
        *found = 18;
        return 18;
    }
    tmp2 = tmp >> 23;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (13 - 4)) == value {
        *found = 19;
        return 19;
    }
    if ((tmp >> 4) & 8191) == value {
        *found = 20;
        return 20;
    }
    if ((tmp >> 17) & 8191) == value {
        *found = 21;
        return 21;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 11)) << (13 - 11)) == value {
        *found = 22;
        return 22;
    }
    if ((tmp >> 11) & 8191) == value {
        *found = 23;
        return 23;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 5)) << (13 - 5)) == value {
        *found = 24;
        return 24;
    }
    if ((tmp >> 5) & 8191) == value {
        *found = 25;
        return 25;
    }
    if ((tmp >> 18) & 8191) == value {
        *found = 26;
        return 26;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (13 - 12)) == value {
        *found = 27;
        return 27;
    }
    if ((tmp >> 12) & 8191) == value {
        *found = 28;
        return 28;
    }
    tmp2 = tmp >> 25;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (13 - 6)) == value {
        *found = 29;
        return 29;
    }
    if ((tmp >> 6) & 8191) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 19) & 8191) == value {
        *found = 31;
        return 31;
    }
    52
}

pub fn linsearch14_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 16383) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 14) & 16383) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (14 - 10)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 10) & 16383) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (14 - 6)) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 6) & 16383) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (14 - 2)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 2) & 16383) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 16) & 16383) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (14 - 12)) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 12) & 16383) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (14 - 8)) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 8) & 16383) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (14 - 4)) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 4) & 16383) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 18) & 16383) == value {
        *found = 15;
        return 15;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 16383) == value {
        *found = 16;
        return 16;
    }
    if ((tmp >> 14) & 16383) == value {
        *found = 17;
        return 17;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (14 - 10)) == value {
        *found = 18;
        return 18;
    }
    if ((tmp >> 10) & 16383) == value {
        *found = 19;
        return 19;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (14 - 6)) == value {
        *found = 20;
        return 20;
    }
    if ((tmp >> 6) & 16383) == value {
        *found = 21;
        return 21;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (14 - 2)) == value {
        *found = 22;
        return 22;
    }
    if ((tmp >> 2) & 16383) == value {
        *found = 23;
        return 23;
    }
    if ((tmp >> 16) & 16383) == value {
        *found = 24;
        return 24;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (14 - 12)) == value {
        *found = 25;
        return 25;
    }
    if ((tmp >> 12) & 16383) == value {
        *found = 26;
        return 26;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (14 - 8)) == value {
        *found = 27;
        return 27;
    }
    if ((tmp >> 8) & 16383) == value {
        *found = 28;
        return 28;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (14 - 4)) == value {
        *found = 29;
        return 29;
    }
    if ((tmp >> 4) & 16383) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 18) & 16383) == value {
        *found = 31;
        return 31;
    }
    56
}

pub fn linsearch15_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 32767) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 15) & 32767) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 13)) << (15 - 13)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 13) & 32767) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 11)) << (15 - 11)) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 11) & 32767) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 9)) << (15 - 9)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 9) & 32767) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 7)) << (15 - 7)) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 7) & 32767) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 5)) << (15 - 5)) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 5) & 32767) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 3)) << (15 - 3)) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 3) & 32767) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (15 - 1)) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 1) & 32767) == value {
        *found = 15;
        return 15;
    }
    if ((tmp >> 16) & 32767) == value {
        *found = 16;
        return 16;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (15 - 14)) == value {
        *found = 17;
        return 17;
    }
    if ((tmp >> 14) & 32767) == value {
        *found = 18;
        return 18;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (15 - 12)) == value {
        *found = 19;
        return 19;
    }
    if ((tmp >> 12) & 32767) == value {
        *found = 20;
        return 20;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (15 - 10)) == value {
        *found = 21;
        return 21;
    }
    if ((tmp >> 10) & 32767) == value {
        *found = 22;
        return 22;
    }
    tmp2 = tmp >> 25;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (15 - 8)) == value {
        *found = 23;
        return 23;
    }
    if ((tmp >> 8) & 32767) == value {
        *found = 24;
        return 24;
    }
    tmp2 = tmp >> 23;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (15 - 6)) == value {
        *found = 25;
        return 25;
    }
    if ((tmp >> 6) & 32767) == value {
        *found = 26;
        return 26;
    }
    tmp2 = tmp >> 21;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (15 - 4)) == value {
        *found = 27;
        return 27;
    }
    if ((tmp >> 4) & 32767) == value {
        *found = 28;
        return 28;
    }
    tmp2 = tmp >> 19;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (15 - 2)) == value {
        *found = 29;
        return 29;
    }
    if ((tmp >> 2) & 32767) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 17) & 32767) == value {
        *found = 31;
        return 31;
    }
    60
}

pub fn linsearch16_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 1;
        return 1;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 3;
        return 3;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 5;
        return 5;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 7;
        return 7;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 9;
        return 9;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 11;
        return 11;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 13;
        return 13;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 15;
        return 15;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 16;
        return 16;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 17;
        return 17;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 18;
        return 18;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 19;
        return 19;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 20;
        return 20;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 21;
        return 21;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 22;
        return 22;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 23;
        return 23;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 24;
        return 24;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 25;
        return 25;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 26;
        return 26;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 27;
        return 27;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 28;
        return 28;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 29;
        return 29;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 31;
        return 31;
    }
    64
}

pub fn linsearch17_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 131071) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 17;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (17 - 2)) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 2) & 131071) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 19;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (17 - 4)) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 4) & 131071) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 21;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (17 - 6)) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 6) & 131071) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 23;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (17 - 8)) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 8) & 131071) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 25;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (17 - 10)) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 10) & 131071) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (17 - 12)) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 12) & 131071) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (17 - 14)) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 14) & 131071) == value {
        *found = 14;
        return 14;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (17 - 16)) == value {
        *found = 15;
        return 15;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (17 - 1)) == value {
        *found = 16;
        return 16;
    }
    if ((tmp >> 1) & 131071) == value {
        *found = 17;
        return 17;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 3)) << (17 - 3)) == value {
        *found = 18;
        return 18;
    }
    if ((tmp >> 3) & 131071) == value {
        *found = 19;
        return 19;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 5)) << (17 - 5)) == value {
        *found = 20;
        return 20;
    }
    if ((tmp >> 5) & 131071) == value {
        *found = 21;
        return 21;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 7)) << (17 - 7)) == value {
        *found = 22;
        return 22;
    }
    if ((tmp >> 7) & 131071) == value {
        *found = 23;
        return 23;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 9)) << (17 - 9)) == value {
        *found = 24;
        return 24;
    }
    if ((tmp >> 9) & 131071) == value {
        *found = 25;
        return 25;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 11)) << (17 - 11)) == value {
        *found = 26;
        return 26;
    }
    if ((tmp >> 11) & 131071) == value {
        *found = 27;
        return 27;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 13)) << (17 - 13)) == value {
        *found = 28;
        return 28;
    }
    if ((tmp >> 13) & 131071) == value {
        *found = 29;
        return 29;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 15)) << (17 - 15)) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 15) & 131071) == value {
        *found = 31;
        return 31;
    }
    68
}

pub fn linsearch18_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 262143) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (18 - 4)) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 4) & 262143) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (18 - 8)) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 8) & 262143) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (18 - 12)) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 12) & 262143) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (18 - 16)) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (18 - 2)) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 2) & 262143) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (18 - 6)) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 6) & 262143) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (18 - 10)) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 10) & 262143) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (18 - 14)) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 14) & 262143) == value {
        *found = 15;
        return 15;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 262143) == value {
        *found = 16;
        return 16;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (18 - 4)) == value {
        *found = 17;
        return 17;
    }
    if ((tmp >> 4) & 262143) == value {
        *found = 18;
        return 18;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (18 - 8)) == value {
        *found = 19;
        return 19;
    }
    if ((tmp >> 8) & 262143) == value {
        *found = 20;
        return 20;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (18 - 12)) == value {
        *found = 21;
        return 21;
    }
    if ((tmp >> 12) & 262143) == value {
        *found = 22;
        return 22;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (18 - 16)) == value {
        *found = 23;
        return 23;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (18 - 2)) == value {
        *found = 24;
        return 24;
    }
    if ((tmp >> 2) & 262143) == value {
        *found = 25;
        return 25;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (18 - 6)) == value {
        *found = 26;
        return 26;
    }
    if ((tmp >> 6) & 262143) == value {
        *found = 27;
        return 27;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (18 - 10)) == value {
        *found = 28;
        return 28;
    }
    if ((tmp >> 10) & 262143) == value {
        *found = 29;
        return 29;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (18 - 14)) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 14) & 262143) == value {
        *found = 31;
        return 31;
    }
    72
}

pub fn linsearch19_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 524287) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 19;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (19 - 6)) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 6) & 524287) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 25;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (19 - 12)) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 12) & 524287) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 18)) << (19 - 18)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 5)) << (19 - 5)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 5) & 524287) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 11)) << (19 - 11)) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 11) & 524287) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 17)) << (19 - 17)) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 17;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (19 - 4)) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 4) & 524287) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 23;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (19 - 10)) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 10) & 524287) == value {
        *found = 14;
        return 14;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (19 - 16)) == value {
        *found = 15;
        return 15;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 3)) << (19 - 3)) == value {
        *found = 16;
        return 16;
    }
    if ((tmp >> 3) & 524287) == value {
        *found = 17;
        return 17;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 9)) << (19 - 9)) == value {
        *found = 18;
        return 18;
    }
    if ((tmp >> 9) & 524287) == value {
        *found = 19;
        return 19;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 15)) << (19 - 15)) == value {
        *found = 20;
        return 20;
    }
    tmp2 = tmp >> 15;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (19 - 2)) == value {
        *found = 21;
        return 21;
    }
    if ((tmp >> 2) & 524287) == value {
        *found = 22;
        return 22;
    }
    tmp2 = tmp >> 21;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (19 - 8)) == value {
        *found = 23;
        return 23;
    }
    if ((tmp >> 8) & 524287) == value {
        *found = 24;
        return 24;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (19 - 14)) == value {
        *found = 25;
        return 25;
    }
    tmp2 = tmp >> 14;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (19 - 1)) == value {
        *found = 26;
        return 26;
    }
    if ((tmp >> 1) & 524287) == value {
        *found = 27;
        return 27;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 7)) << (19 - 7)) == value {
        *found = 28;
        return 28;
    }
    if ((tmp >> 7) & 524287) == value {
        *found = 29;
        return 29;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 13)) << (19 - 13)) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 13) & 524287) == value {
        *found = 31;
        return 31;
    }
    76
}

pub fn linsearch20_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 1048575) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (20 - 8)) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 8) & 1048575) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (20 - 16)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (20 - 4)) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 4) & 1048575) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (20 - 12)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 12) & 1048575) == value {
        *found = 7;
        return 7;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 1048575) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (20 - 8)) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 8) & 1048575) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (20 - 16)) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (20 - 4)) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 4) & 1048575) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (20 - 12)) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 12) & 1048575) == value {
        *found = 15;
        return 15;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 1048575) == value {
        *found = 16;
        return 16;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (20 - 8)) == value {
        *found = 17;
        return 17;
    }
    if ((tmp >> 8) & 1048575) == value {
        *found = 18;
        return 18;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (20 - 16)) == value {
        *found = 19;
        return 19;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (20 - 4)) == value {
        *found = 20;
        return 20;
    }
    if ((tmp >> 4) & 1048575) == value {
        *found = 21;
        return 21;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (20 - 12)) == value {
        *found = 22;
        return 22;
    }
    if ((tmp >> 12) & 1048575) == value {
        *found = 23;
        return 23;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 1048575) == value {
        *found = 24;
        return 24;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (20 - 8)) == value {
        *found = 25;
        return 25;
    }
    if ((tmp >> 8) & 1048575) == value {
        *found = 26;
        return 26;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (20 - 16)) == value {
        *found = 27;
        return 27;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (20 - 4)) == value {
        *found = 28;
        return 28;
    }
    if ((tmp >> 4) & 1048575) == value {
        *found = 29;
        return 29;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (20 - 12)) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 12) & 1048575) == value {
        *found = 31;
        return 31;
    }
    80
}

pub fn linsearch21_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 2097151) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 21;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (21 - 10)) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 10) & 2097151) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (21 - 20)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 9)) << (21 - 9)) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 9) & 2097151) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 19)) << (21 - 19)) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 19;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (21 - 8)) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 8) & 2097151) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 18)) << (21 - 18)) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 7)) << (21 - 7)) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 7) & 2097151) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 17)) << (21 - 17)) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 17;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (21 - 6)) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 6) & 2097151) == value {
        *found = 14;
        return 14;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (21 - 16)) == value {
        *found = 15;
        return 15;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 5)) << (21 - 5)) == value {
        *found = 16;
        return 16;
    }
    if ((tmp >> 5) & 2097151) == value {
        *found = 17;
        return 17;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 15)) << (21 - 15)) == value {
        *found = 18;
        return 18;
    }
    tmp2 = tmp >> 15;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (21 - 4)) == value {
        *found = 19;
        return 19;
    }
    if ((tmp >> 4) & 2097151) == value {
        *found = 20;
        return 20;
    }
    tmp2 = tmp >> 25;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (21 - 14)) == value {
        *found = 21;
        return 21;
    }
    tmp2 = tmp >> 14;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 3)) << (21 - 3)) == value {
        *found = 22;
        return 22;
    }
    if ((tmp >> 3) & 2097151) == value {
        *found = 23;
        return 23;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 13)) << (21 - 13)) == value {
        *found = 24;
        return 24;
    }
    tmp2 = tmp >> 13;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (21 - 2)) == value {
        *found = 25;
        return 25;
    }
    if ((tmp >> 2) & 2097151) == value {
        *found = 26;
        return 26;
    }
    tmp2 = tmp >> 23;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (21 - 12)) == value {
        *found = 27;
        return 27;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (21 - 1)) == value {
        *found = 28;
        return 28;
    }
    if ((tmp >> 1) & 2097151) == value {
        *found = 29;
        return 29;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 11)) << (21 - 11)) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 11) & 2097151) == value {
        *found = 31;
        return 31;
    }
    84
}

pub fn linsearch22_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 4194303) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (22 - 12)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (22 - 2)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 2) & 4194303) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (22 - 14)) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 14;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (22 - 4)) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 4) & 4194303) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (22 - 16)) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (22 - 6)) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 6) & 4194303) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 18)) << (22 - 18)) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (22 - 8)) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 8) & 4194303) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (22 - 20)) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (22 - 10)) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 10) & 4194303) == value {
        *found = 15;
        return 15;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 4194303) == value {
        *found = 16;
        return 16;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (22 - 12)) == value {
        *found = 17;
        return 17;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (22 - 2)) == value {
        *found = 18;
        return 18;
    }
    if ((tmp >> 2) & 4194303) == value {
        *found = 19;
        return 19;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (22 - 14)) == value {
        *found = 20;
        return 20;
    }
    tmp2 = tmp >> 14;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (22 - 4)) == value {
        *found = 21;
        return 21;
    }
    if ((tmp >> 4) & 4194303) == value {
        *found = 22;
        return 22;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (22 - 16)) == value {
        *found = 23;
        return 23;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (22 - 6)) == value {
        *found = 24;
        return 24;
    }
    if ((tmp >> 6) & 4194303) == value {
        *found = 25;
        return 25;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 18)) << (22 - 18)) == value {
        *found = 26;
        return 26;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (22 - 8)) == value {
        *found = 27;
        return 27;
    }
    if ((tmp >> 8) & 4194303) == value {
        *found = 28;
        return 28;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (22 - 20)) == value {
        *found = 29;
        return 29;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (22 - 10)) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 10) & 4194303) == value {
        *found = 31;
        return 31;
    }
    88
}

pub fn linsearch23_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 8388607) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 23;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (23 - 14)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 14;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 5)) << (23 - 5)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 5) & 8388607) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 19)) << (23 - 19)) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 19;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (23 - 10)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 10;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (23 - 1)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 1) & 8388607) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 15)) << (23 - 15)) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 15;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (23 - 6)) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 6) & 8388607) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (23 - 20)) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 11)) << (23 - 11)) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 11;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (23 - 2)) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 2) & 8388607) == value {
        *found = 14;
        return 14;
    }
    tmp2 = tmp >> 25;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (23 - 16)) == value {
        *found = 15;
        return 15;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 7)) << (23 - 7)) == value {
        *found = 16;
        return 16;
    }
    if ((tmp >> 7) & 8388607) == value {
        *found = 17;
        return 17;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 21)) << (23 - 21)) == value {
        *found = 18;
        return 18;
    }
    tmp2 = tmp >> 21;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (23 - 12)) == value {
        *found = 19;
        return 19;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 3)) << (23 - 3)) == value {
        *found = 20;
        return 20;
    }
    if ((tmp >> 3) & 8388607) == value {
        *found = 21;
        return 21;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 17)) << (23 - 17)) == value {
        *found = 22;
        return 22;
    }
    tmp2 = tmp >> 17;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (23 - 8)) == value {
        *found = 23;
        return 23;
    }
    if ((tmp >> 8) & 8388607) == value {
        *found = 24;
        return 24;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 22)) << (23 - 22)) == value {
        *found = 25;
        return 25;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 13)) << (23 - 13)) == value {
        *found = 26;
        return 26;
    }
    tmp2 = tmp >> 13;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (23 - 4)) == value {
        *found = 27;
        return 27;
    }
    if ((tmp >> 4) & 8388607) == value {
        *found = 28;
        return 28;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 18)) << (23 - 18)) == value {
        *found = 29;
        return 29;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 9)) << (23 - 9)) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 9) & 8388607) == value {
        *found = 31;
        return 31;
    }
    92
}

pub fn linsearch24_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 16777215) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (24 - 16)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (24 - 8)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 8) & 16777215) == value {
        *found = 3;
        return 3;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 16777215) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (24 - 16)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (24 - 8)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 8) & 16777215) == value {
        *found = 7;
        return 7;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 16777215) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (24 - 16)) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (24 - 8)) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 8) & 16777215) == value {
        *found = 11;
        return 11;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 16777215) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (24 - 16)) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (24 - 8)) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 8) & 16777215) == value {
        *found = 15;
        return 15;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 16777215) == value {
        *found = 16;
        return 16;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (24 - 16)) == value {
        *found = 17;
        return 17;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (24 - 8)) == value {
        *found = 18;
        return 18;
    }
    if ((tmp >> 8) & 16777215) == value {
        *found = 19;
        return 19;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 16777215) == value {
        *found = 20;
        return 20;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (24 - 16)) == value {
        *found = 21;
        return 21;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (24 - 8)) == value {
        *found = 22;
        return 22;
    }
    if ((tmp >> 8) & 16777215) == value {
        *found = 23;
        return 23;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 16777215) == value {
        *found = 24;
        return 24;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (24 - 16)) == value {
        *found = 25;
        return 25;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (24 - 8)) == value {
        *found = 26;
        return 26;
    }
    if ((tmp >> 8) & 16777215) == value {
        *found = 27;
        return 27;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 16777215) == value {
        *found = 28;
        return 28;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (24 - 16)) == value {
        *found = 29;
        return 29;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (24 - 8)) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 8) & 16777215) == value {
        *found = 31;
        return 31;
    }
    96
}

pub fn linsearch25_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 33554431) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 25;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 18)) << (25 - 18)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 11)) << (25 - 11)) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 11;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (25 - 4)) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 4) & 33554431) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 22)) << (25 - 22)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 15)) << (25 - 15)) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 15;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (25 - 8)) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 8;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (25 - 1)) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 1) & 33554431) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 19)) << (25 - 19)) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 19;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (25 - 12)) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 5)) << (25 - 5)) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 5) & 33554431) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 23)) << (25 - 23)) == value {
        *found = 14;
        return 14;
    }
    tmp2 = tmp >> 23;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (25 - 16)) == value {
        *found = 15;
        return 15;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 9)) << (25 - 9)) == value {
        *found = 16;
        return 16;
    }
    tmp2 = tmp >> 9;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (25 - 2)) == value {
        *found = 17;
        return 17;
    }
    if ((tmp >> 2) & 33554431) == value {
        *found = 18;
        return 18;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (25 - 20)) == value {
        *found = 19;
        return 19;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 13)) << (25 - 13)) == value {
        *found = 20;
        return 20;
    }
    tmp2 = tmp >> 13;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (25 - 6)) == value {
        *found = 21;
        return 21;
    }
    if ((tmp >> 6) & 33554431) == value {
        *found = 22;
        return 22;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 24)) << (25 - 24)) == value {
        *found = 23;
        return 23;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 17)) << (25 - 17)) == value {
        *found = 24;
        return 24;
    }
    tmp2 = tmp >> 17;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (25 - 10)) == value {
        *found = 25;
        return 25;
    }
    tmp2 = tmp >> 10;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 3)) << (25 - 3)) == value {
        *found = 26;
        return 26;
    }
    if ((tmp >> 3) & 33554431) == value {
        *found = 27;
        return 27;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 21)) << (25 - 21)) == value {
        *found = 28;
        return 28;
    }
    tmp2 = tmp >> 21;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (25 - 14)) == value {
        *found = 29;
        return 29;
    }
    tmp2 = tmp >> 14;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 7)) << (25 - 7)) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 7) & 33554431) == value {
        *found = 31;
        return 31;
    }
    100
}

pub fn linsearch26_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 67108863) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (26 - 20)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (26 - 14)) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 14;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (26 - 8)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 8;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (26 - 2)) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 2) & 67108863) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 22)) << (26 - 22)) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (26 - 16)) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (26 - 10)) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 10;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (26 - 4)) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 4) & 67108863) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 24)) << (26 - 24)) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 18)) << (26 - 18)) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (26 - 12)) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (26 - 6)) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 6) & 67108863) == value {
        *found = 15;
        return 15;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 67108863) == value {
        *found = 16;
        return 16;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (26 - 20)) == value {
        *found = 17;
        return 17;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (26 - 14)) == value {
        *found = 18;
        return 18;
    }
    tmp2 = tmp >> 14;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (26 - 8)) == value {
        *found = 19;
        return 19;
    }
    tmp2 = tmp >> 8;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (26 - 2)) == value {
        *found = 20;
        return 20;
    }
    if ((tmp >> 2) & 67108863) == value {
        *found = 21;
        return 21;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 22)) << (26 - 22)) == value {
        *found = 22;
        return 22;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (26 - 16)) == value {
        *found = 23;
        return 23;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (26 - 10)) == value {
        *found = 24;
        return 24;
    }
    tmp2 = tmp >> 10;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (26 - 4)) == value {
        *found = 25;
        return 25;
    }
    if ((tmp >> 4) & 67108863) == value {
        *found = 26;
        return 26;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 24)) << (26 - 24)) == value {
        *found = 27;
        return 27;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 18)) << (26 - 18)) == value {
        *found = 28;
        return 28;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (26 - 12)) == value {
        *found = 29;
        return 29;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (26 - 6)) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 6) & 67108863) == value {
        *found = 31;
        return 31;
    }
    104
}

pub fn linsearch27_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 134217727) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 22)) << (27 - 22)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 17)) << (27 - 17)) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 17;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (27 - 12)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 7)) << (27 - 7)) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 7;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (27 - 2)) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 2) & 134217727) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 24)) << (27 - 24)) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 19)) << (27 - 19)) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 19;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (27 - 14)) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 14;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 9)) << (27 - 9)) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 9;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (27 - 4)) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 4) & 134217727) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 26)) << (27 - 26)) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 21)) << (27 - 21)) == value {
        *found = 14;
        return 14;
    }
    tmp2 = tmp >> 21;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (27 - 16)) == value {
        *found = 15;
        return 15;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 11)) << (27 - 11)) == value {
        *found = 16;
        return 16;
    }
    tmp2 = tmp >> 11;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (27 - 6)) == value {
        *found = 17;
        return 17;
    }
    tmp2 = tmp >> 6;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (27 - 1)) == value {
        *found = 18;
        return 18;
    }
    if ((tmp >> 1) & 134217727) == value {
        *found = 19;
        return 19;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 23)) << (27 - 23)) == value {
        *found = 20;
        return 20;
    }
    tmp2 = tmp >> 23;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 18)) << (27 - 18)) == value {
        *found = 21;
        return 21;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 13)) << (27 - 13)) == value {
        *found = 22;
        return 22;
    }
    tmp2 = tmp >> 13;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (27 - 8)) == value {
        *found = 23;
        return 23;
    }
    tmp2 = tmp >> 8;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 3)) << (27 - 3)) == value {
        *found = 24;
        return 24;
    }
    if ((tmp >> 3) & 134217727) == value {
        *found = 25;
        return 25;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 25)) << (27 - 25)) == value {
        *found = 26;
        return 26;
    }
    tmp2 = tmp >> 25;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (27 - 20)) == value {
        *found = 27;
        return 27;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 15)) << (27 - 15)) == value {
        *found = 28;
        return 28;
    }
    tmp2 = tmp >> 15;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (27 - 10)) == value {
        *found = 29;
        return 29;
    }
    tmp2 = tmp >> 10;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 5)) << (27 - 5)) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 5) & 134217727) == value {
        *found = 31;
        return 31;
    }
    108
}

pub fn linsearch28_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 268435455) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 24)) << (28 - 24)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (28 - 20)) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (28 - 16)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (28 - 12)) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (28 - 8)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 8;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (28 - 4)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 4) & 268435455) == value {
        *found = 7;
        return 7;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 268435455) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 24)) << (28 - 24)) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (28 - 20)) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (28 - 16)) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (28 - 12)) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (28 - 8)) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 8;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (28 - 4)) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 4) & 268435455) == value {
        *found = 15;
        return 15;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 268435455) == value {
        *found = 16;
        return 16;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 24)) << (28 - 24)) == value {
        *found = 17;
        return 17;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (28 - 20)) == value {
        *found = 18;
        return 18;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (28 - 16)) == value {
        *found = 19;
        return 19;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (28 - 12)) == value {
        *found = 20;
        return 20;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (28 - 8)) == value {
        *found = 21;
        return 21;
    }
    tmp2 = tmp >> 8;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (28 - 4)) == value {
        *found = 22;
        return 22;
    }
    if ((tmp >> 4) & 268435455) == value {
        *found = 23;
        return 23;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 268435455) == value {
        *found = 24;
        return 24;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 24)) << (28 - 24)) == value {
        *found = 25;
        return 25;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (28 - 20)) == value {
        *found = 26;
        return 26;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (28 - 16)) == value {
        *found = 27;
        return 27;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (28 - 12)) == value {
        *found = 28;
        return 28;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (28 - 8)) == value {
        *found = 29;
        return 29;
    }
    tmp2 = tmp >> 8;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (28 - 4)) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 4) & 268435455) == value {
        *found = 31;
        return 31;
    }
    112
}

pub fn linsearch29_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 536870911) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 26)) << (29 - 26)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 23)) << (29 - 23)) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 23;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (29 - 20)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 17)) << (29 - 17)) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 17;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (29 - 14)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 14;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 11)) << (29 - 11)) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 11;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (29 - 8)) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 8;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 5)) << (29 - 5)) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 5;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (29 - 2)) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 2) & 536870911) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 28)) << (29 - 28)) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 25)) << (29 - 25)) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 25;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 22)) << (29 - 22)) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 19)) << (29 - 19)) == value {
        *found = 14;
        return 14;
    }
    tmp2 = tmp >> 19;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (29 - 16)) == value {
        *found = 15;
        return 15;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 13)) << (29 - 13)) == value {
        *found = 16;
        return 16;
    }
    tmp2 = tmp >> 13;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (29 - 10)) == value {
        *found = 17;
        return 17;
    }
    tmp2 = tmp >> 10;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 7)) << (29 - 7)) == value {
        *found = 18;
        return 18;
    }
    tmp2 = tmp >> 7;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (29 - 4)) == value {
        *found = 19;
        return 19;
    }
    tmp2 = tmp >> 4;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (29 - 1)) == value {
        *found = 20;
        return 20;
    }
    if ((tmp >> 1) & 536870911) == value {
        *found = 21;
        return 21;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 27)) << (29 - 27)) == value {
        *found = 22;
        return 22;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 24)) << (29 - 24)) == value {
        *found = 23;
        return 23;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 21)) << (29 - 21)) == value {
        *found = 24;
        return 24;
    }
    tmp2 = tmp >> 21;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 18)) << (29 - 18)) == value {
        *found = 25;
        return 25;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 15)) << (29 - 15)) == value {
        *found = 26;
        return 26;
    }
    tmp2 = tmp >> 15;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (29 - 12)) == value {
        *found = 27;
        return 27;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 9)) << (29 - 9)) == value {
        *found = 28;
        return 28;
    }
    tmp2 = tmp >> 9;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (29 - 6)) == value {
        *found = 29;
        return 29;
    }
    tmp2 = tmp >> 6;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 3)) << (29 - 3)) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 3) & 536870911) == value {
        *found = 31;
        return 31;
    }
    116
}

pub fn linsearch30_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 1073741823) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 28)) << (30 - 28)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 26)) << (30 - 26)) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 24)) << (30 - 24)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 22)) << (30 - 22)) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (30 - 20)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 18)) << (30 - 18)) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (30 - 16)) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (30 - 14)) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 14;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (30 - 12)) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (30 - 10)) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 10;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (30 - 8)) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 8;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (30 - 6)) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 6;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (30 - 4)) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 4;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (30 - 2)) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 2) & 1073741823) == value {
        *found = 15;
        return 15;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 1073741823) == value {
        *found = 16;
        return 16;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 28)) << (30 - 28)) == value {
        *found = 17;
        return 17;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 26)) << (30 - 26)) == value {
        *found = 18;
        return 18;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 24)) << (30 - 24)) == value {
        *found = 19;
        return 19;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 22)) << (30 - 22)) == value {
        *found = 20;
        return 20;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (30 - 20)) == value {
        *found = 21;
        return 21;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 18)) << (30 - 18)) == value {
        *found = 22;
        return 22;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (30 - 16)) == value {
        *found = 23;
        return 23;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (30 - 14)) == value {
        *found = 24;
        return 24;
    }
    tmp2 = tmp >> 14;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (30 - 12)) == value {
        *found = 25;
        return 25;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (30 - 10)) == value {
        *found = 26;
        return 26;
    }
    tmp2 = tmp >> 10;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (30 - 8)) == value {
        *found = 27;
        return 27;
    }
    tmp2 = tmp >> 8;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (30 - 6)) == value {
        *found = 28;
        return 28;
    }
    tmp2 = tmp >> 6;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (30 - 4)) == value {
        *found = 29;
        return 29;
    }
    tmp2 = tmp >> 4;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (30 - 2)) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 2) & 1073741823) == value {
        *found = 31;
        return 31;
    }
    120
}

pub fn linsearch31_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 2147483647) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 30)) << (31 - 30)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 29)) << (31 - 29)) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 28)) << (31 - 28)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 27)) << (31 - 27)) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 26)) << (31 - 26)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 25)) << (31 - 25)) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 25;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 24)) << (31 - 24)) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 23)) << (31 - 23)) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 23;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 22)) << (31 - 22)) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 21)) << (31 - 21)) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 21;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (31 - 20)) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 19)) << (31 - 19)) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 19;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 18)) << (31 - 18)) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 17)) << (31 - 17)) == value {
        *found = 14;
        return 14;
    }
    tmp2 = tmp >> 17;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (31 - 16)) == value {
        *found = 15;
        return 15;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 15)) << (31 - 15)) == value {
        *found = 16;
        return 16;
    }
    tmp2 = tmp >> 15;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (31 - 14)) == value {
        *found = 17;
        return 17;
    }
    tmp2 = tmp >> 14;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 13)) << (31 - 13)) == value {
        *found = 18;
        return 18;
    }
    tmp2 = tmp >> 13;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (31 - 12)) == value {
        *found = 19;
        return 19;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 11)) << (31 - 11)) == value {
        *found = 20;
        return 20;
    }
    tmp2 = tmp >> 11;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (31 - 10)) == value {
        *found = 21;
        return 21;
    }
    tmp2 = tmp >> 10;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 9)) << (31 - 9)) == value {
        *found = 22;
        return 22;
    }
    tmp2 = tmp >> 9;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (31 - 8)) == value {
        *found = 23;
        return 23;
    }
    tmp2 = tmp >> 8;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 7)) << (31 - 7)) == value {
        *found = 24;
        return 24;
    }
    tmp2 = tmp >> 7;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (31 - 6)) == value {
        *found = 25;
        return 25;
    }
    tmp2 = tmp >> 6;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 5)) << (31 - 5)) == value {
        *found = 26;
        return 26;
    }
    tmp2 = tmp >> 5;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (31 - 4)) == value {
        *found = 27;
        return 27;
    }
    tmp2 = tmp >> 4;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 3)) << (31 - 3)) == value {
        *found = 28;
        return 28;
    }
    tmp2 = tmp >> 3;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (31 - 2)) == value {
        *found = 29;
        return 29;
    }
    tmp2 = tmp >> 2;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (31 - 1)) == value {
        *found = 30;
        return 30;
    }
    if ((tmp >> 1) & 2147483647) == value {
        *found = 31;
        return 31;
    }
    124
}

pub fn linsearch32_32(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    for i in 0..32 {
        if read_u32(input, i * 4) == value {
            *found = i as i32;
            return 0;
        }
    }
    128
}

pub fn linsearch1_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 1) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 1) & 1) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 2) & 1) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 3) & 1) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 4) & 1) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 5) & 1) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 6) & 1) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 7) & 1) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 8) & 1) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 9) & 1) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 10) & 1) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 11) & 1) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 12) & 1) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 13) & 1) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 14) & 1) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 15) & 1) == value {
        *found = 15;
        return 15;
    }
    2
}

pub fn linsearch2_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 3) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 2) & 3) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 4) & 3) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 6) & 3) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 8) & 3) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 10) & 3) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 12) & 3) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 14) & 3) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 16) & 3) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 18) & 3) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 20) & 3) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 22) & 3) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 24) & 3) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 26) & 3) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 28) & 3) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 30) & 3) == value {
        *found = 15;
        return 15;
    }
    4
}

pub fn linsearch3_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 7) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 3) & 7) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 6) & 7) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 9) & 7) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 12) & 7) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 15) & 7) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 18) & 7) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 21) & 7) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 24) & 7) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 27) & 7) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (3 - 1)) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 1) & 7) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 4) & 7) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 7) & 7) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 10) & 7) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 13) & 7) == value {
        *found = 15;
        return 15;
    }
    6
}

pub fn linsearch4_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 15) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 4) & 15) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 8) & 15) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 12) & 15) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 16) & 15) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 20) & 15) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 24) & 15) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 28) & 15) == value {
        *found = 7;
        return 7;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 15) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 4) & 15) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 8) & 15) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 12) & 15) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 16) & 15) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 20) & 15) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 24) & 15) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 28) & 15) == value {
        *found = 15;
        return 15;
    }
    8
}

pub fn linsearch5_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 31) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 5) & 31) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 10) & 31) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 15) & 31) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 20) & 31) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 25) & 31) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 3)) << (5 - 3)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 3) & 31) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 8) & 31) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 13) & 31) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 18) & 31) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 23) & 31) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (5 - 1)) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 1) & 31) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 6) & 31) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 11) & 31) == value {
        *found = 15;
        return 15;
    }
    10
}

pub fn linsearch6_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 63) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 6) & 63) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 12) & 63) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 18) & 63) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 24) & 63) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (6 - 4)) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 4) & 63) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 10) & 63) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 16) & 63) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 22) & 63) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (6 - 2)) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 2) & 63) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 8) & 63) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 14) & 63) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 20) & 63) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 26) & 63) == value {
        *found = 15;
        return 15;
    }
    12
}

pub fn linsearch7_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 127) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 7) & 127) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 14) & 127) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 21) & 127) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 3)) << (7 - 3)) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 3) & 127) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 10) & 127) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 17) & 127) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 24) & 127) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (7 - 6)) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 6) & 127) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 13) & 127) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 20) & 127) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (7 - 2)) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 2) & 127) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 9) & 127) == value {
        *found = 15;
        return 15;
    }
    14
}

pub fn linsearch8_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 255) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 8) & 255) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 16) & 255) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 24) & 255) == value {
        *found = 3;
        return 3;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 255) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 8) & 255) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 16) & 255) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 24) & 255) == value {
        *found = 7;
        return 7;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 255) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 8) & 255) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 16) & 255) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 24) & 255) == value {
        *found = 11;
        return 11;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 255) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 8) & 255) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 16) & 255) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 24) & 255) == value {
        *found = 15;
        return 15;
    }
    16
}

pub fn linsearch9_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 511) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 9) & 511) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 18) & 511) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (9 - 4)) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 4) & 511) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 13) & 511) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 22) & 511) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (9 - 8)) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 8) & 511) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 17) & 511) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 3)) << (9 - 3)) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 3) & 511) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 12) & 511) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 21) & 511) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 7)) << (9 - 7)) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 7) & 511) == value {
        *found = 15;
        return 15;
    }
    18
}

pub fn linsearch10_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 1023) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 10) & 1023) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 20) & 1023) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (10 - 8)) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 8) & 1023) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 18) & 1023) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (10 - 6)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 6) & 1023) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 16) & 1023) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (10 - 4)) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 4) & 1023) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 14) & 1023) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (10 - 2)) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 2) & 1023) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 12) & 1023) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 22) & 1023) == value {
        *found = 15;
        return 15;
    }
    20
}

pub fn linsearch11_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 2047) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 11) & 2047) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (11 - 1)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 1) & 2047) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 12) & 2047) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 23;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (11 - 2)) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 2) & 2047) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 13) & 2047) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 3)) << (11 - 3)) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 3) & 2047) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 14) & 2047) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 25;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (11 - 4)) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 4) & 2047) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 15) & 2047) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 5)) << (11 - 5)) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 5) & 2047) == value {
        *found = 15;
        return 15;
    }
    22
}

pub fn linsearch12_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 4095) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 12) & 4095) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (12 - 4)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 4) & 4095) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 16) & 4095) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (12 - 8)) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 8) & 4095) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 20) & 4095) == value {
        *found = 7;
        return 7;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 4095) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 12) & 4095) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (12 - 4)) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 4) & 4095) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 16) & 4095) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (12 - 8)) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 8) & 4095) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 20) & 4095) == value {
        *found = 15;
        return 15;
    }
    24
}

pub fn linsearch13_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 8191) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 13) & 8191) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 7)) << (13 - 7)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 7) & 8191) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (13 - 1)) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 1) & 8191) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 14) & 8191) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (13 - 8)) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 8) & 8191) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 21;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (13 - 2)) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 2) & 8191) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 15) & 8191) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 9)) << (13 - 9)) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 9) & 8191) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 3)) << (13 - 3)) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 3) & 8191) == value {
        *found = 15;
        return 15;
    }
    26
}

pub fn linsearch14_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 16383) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 14) & 16383) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (14 - 10)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 10) & 16383) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (14 - 6)) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 6) & 16383) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (14 - 2)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 2) & 16383) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 16) & 16383) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (14 - 12)) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 12) & 16383) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (14 - 8)) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 8) & 16383) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (14 - 4)) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 4) & 16383) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 18) & 16383) == value {
        *found = 15;
        return 15;
    }
    28
}

pub fn linsearch15_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 32767) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 15) & 32767) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 13)) << (15 - 13)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 13) & 32767) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 11)) << (15 - 11)) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 11) & 32767) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 9)) << (15 - 9)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 9) & 32767) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 7)) << (15 - 7)) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 7) & 32767) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 5)) << (15 - 5)) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 5) & 32767) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 3)) << (15 - 3)) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 3) & 32767) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (15 - 1)) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 1) & 32767) == value {
        *found = 15;
        return 15;
    }
    30
}

pub fn linsearch16_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 1;
        return 1;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 3;
        return 3;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 5;
        return 5;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 7;
        return 7;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 9;
        return 9;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 11;
        return 11;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 13;
        return 13;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 15;
        return 15;
    }
    32
}

pub fn linsearch17_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 131071) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 17;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (17 - 2)) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 2) & 131071) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 19;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (17 - 4)) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 4) & 131071) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 21;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (17 - 6)) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 6) & 131071) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 23;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (17 - 8)) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 8) & 131071) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 25;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (17 - 10)) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 10) & 131071) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (17 - 12)) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 12) & 131071) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (17 - 14)) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 14) & 131071) == value {
        *found = 14;
        return 14;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (17 - 16)) == value {
        *found = 15;
        return 15;
    }
    34
}

pub fn linsearch18_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 262143) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (18 - 4)) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 4) & 262143) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (18 - 8)) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 8) & 262143) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (18 - 12)) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 12) & 262143) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (18 - 16)) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (18 - 2)) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 2) & 262143) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (18 - 6)) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 6) & 262143) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (18 - 10)) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 10) & 262143) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (18 - 14)) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 14) & 262143) == value {
        *found = 15;
        return 15;
    }
    36
}

pub fn linsearch19_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 524287) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 19;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (19 - 6)) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 6) & 524287) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 25;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (19 - 12)) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 12) & 524287) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 18)) << (19 - 18)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 5)) << (19 - 5)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 5) & 524287) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 11)) << (19 - 11)) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 11) & 524287) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 17)) << (19 - 17)) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 17;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (19 - 4)) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 4) & 524287) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 23;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (19 - 10)) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 10) & 524287) == value {
        *found = 14;
        return 14;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (19 - 16)) == value {
        *found = 15;
        return 15;
    }
    38
}

pub fn linsearch20_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 1048575) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (20 - 8)) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 8) & 1048575) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (20 - 16)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (20 - 4)) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 4) & 1048575) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (20 - 12)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 12) & 1048575) == value {
        *found = 7;
        return 7;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 1048575) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (20 - 8)) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 8) & 1048575) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (20 - 16)) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (20 - 4)) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 4) & 1048575) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (20 - 12)) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 12) & 1048575) == value {
        *found = 15;
        return 15;
    }
    40
}

pub fn linsearch21_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 2097151) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 21;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (21 - 10)) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 10) & 2097151) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (21 - 20)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 9)) << (21 - 9)) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 9) & 2097151) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 19)) << (21 - 19)) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 19;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (21 - 8)) == value {
        *found = 7;
        return 7;
    }
    if ((tmp >> 8) & 2097151) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 18)) << (21 - 18)) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 7)) << (21 - 7)) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 7) & 2097151) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 17)) << (21 - 17)) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 17;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (21 - 6)) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 6) & 2097151) == value {
        *found = 14;
        return 14;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (21 - 16)) == value {
        *found = 15;
        return 15;
    }
    42
}

pub fn linsearch22_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 4194303) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (22 - 12)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (22 - 2)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 2) & 4194303) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (22 - 14)) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 14;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (22 - 4)) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 4) & 4194303) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (22 - 16)) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (22 - 6)) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 6) & 4194303) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 18)) << (22 - 18)) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (22 - 8)) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 8) & 4194303) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (22 - 20)) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (22 - 10)) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 10) & 4194303) == value {
        *found = 15;
        return 15;
    }
    44
}

pub fn linsearch23_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 8388607) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 23;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (23 - 14)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 14;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 5)) << (23 - 5)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 5) & 8388607) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 19)) << (23 - 19)) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 19;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (23 - 10)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 10;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (23 - 1)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 1) & 8388607) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 15)) << (23 - 15)) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 15;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (23 - 6)) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 6) & 8388607) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (23 - 20)) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 11)) << (23 - 11)) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 11;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (23 - 2)) == value {
        *found = 13;
        return 13;
    }
    if ((tmp >> 2) & 8388607) == value {
        *found = 14;
        return 14;
    }
    tmp2 = tmp >> 25;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (23 - 16)) == value {
        *found = 15;
        return 15;
    }
    46
}

pub fn linsearch24_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 16777215) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (24 - 16)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (24 - 8)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 8) & 16777215) == value {
        *found = 3;
        return 3;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 16777215) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (24 - 16)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (24 - 8)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 8) & 16777215) == value {
        *found = 7;
        return 7;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 16777215) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (24 - 16)) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (24 - 8)) == value {
        *found = 10;
        return 10;
    }
    if ((tmp >> 8) & 16777215) == value {
        *found = 11;
        return 11;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 16777215) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (24 - 16)) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (24 - 8)) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 8) & 16777215) == value {
        *found = 15;
        return 15;
    }
    48
}

pub fn linsearch25_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 33554431) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 25;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 18)) << (25 - 18)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 11)) << (25 - 11)) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 11;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (25 - 4)) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 4) & 33554431) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 22)) << (25 - 22)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 15)) << (25 - 15)) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 15;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (25 - 8)) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 8;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (25 - 1)) == value {
        *found = 8;
        return 8;
    }
    if ((tmp >> 1) & 33554431) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 19)) << (25 - 19)) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 19;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (25 - 12)) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 5)) << (25 - 5)) == value {
        *found = 12;
        return 12;
    }
    if ((tmp >> 5) & 33554431) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 23)) << (25 - 23)) == value {
        *found = 14;
        return 14;
    }
    tmp2 = tmp >> 23;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (25 - 16)) == value {
        *found = 15;
        return 15;
    }
    50
}

pub fn linsearch26_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 67108863) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (26 - 20)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (26 - 14)) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 14;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (26 - 8)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 8;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (26 - 2)) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 2) & 67108863) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 22)) << (26 - 22)) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (26 - 16)) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (26 - 10)) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 10;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (26 - 4)) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 4) & 67108863) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 24)) << (26 - 24)) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 18)) << (26 - 18)) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (26 - 12)) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (26 - 6)) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 6) & 67108863) == value {
        *found = 15;
        return 15;
    }
    52
}

pub fn linsearch27_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 134217727) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 22)) << (27 - 22)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 17)) << (27 - 17)) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 17;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (27 - 12)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 7)) << (27 - 7)) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 7;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (27 - 2)) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 2) & 134217727) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 24)) << (27 - 24)) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 19)) << (27 - 19)) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 19;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (27 - 14)) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 14;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 9)) << (27 - 9)) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 9;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (27 - 4)) == value {
        *found = 11;
        return 11;
    }
    if ((tmp >> 4) & 134217727) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 26)) << (27 - 26)) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 21)) << (27 - 21)) == value {
        *found = 14;
        return 14;
    }
    tmp2 = tmp >> 21;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (27 - 16)) == value {
        *found = 15;
        return 15;
    }
    54
}

pub fn linsearch28_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 268435455) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 24)) << (28 - 24)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (28 - 20)) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (28 - 16)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (28 - 12)) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (28 - 8)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 8;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (28 - 4)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 4) & 268435455) == value {
        *found = 7;
        return 7;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 268435455) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 24)) << (28 - 24)) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (28 - 20)) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (28 - 16)) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (28 - 12)) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (28 - 8)) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 8;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (28 - 4)) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 4) & 268435455) == value {
        *found = 15;
        return 15;
    }
    56
}

pub fn linsearch29_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 536870911) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 26)) << (29 - 26)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 23)) << (29 - 23)) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 23;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (29 - 20)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 17)) << (29 - 17)) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 17;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (29 - 14)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 14;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 11)) << (29 - 11)) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 11;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (29 - 8)) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 8;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 5)) << (29 - 5)) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 5;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (29 - 2)) == value {
        *found = 9;
        return 9;
    }
    if ((tmp >> 2) & 536870911) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 28)) << (29 - 28)) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 25)) << (29 - 25)) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 25;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 22)) << (29 - 22)) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 19)) << (29 - 19)) == value {
        *found = 14;
        return 14;
    }
    tmp2 = tmp >> 19;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (29 - 16)) == value {
        *found = 15;
        return 15;
    }
    58
}

pub fn linsearch30_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 1073741823) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 28)) << (30 - 28)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 26)) << (30 - 26)) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 24)) << (30 - 24)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 22)) << (30 - 22)) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (30 - 20)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 18)) << (30 - 18)) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (30 - 16)) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (30 - 14)) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 14;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (30 - 12)) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (30 - 10)) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 10;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (30 - 8)) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 8;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (30 - 6)) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 6;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (30 - 4)) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 4;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (30 - 2)) == value {
        *found = 14;
        return 14;
    }
    if ((tmp >> 2) & 1073741823) == value {
        *found = 15;
        return 15;
    }
    60
}

pub fn linsearch31_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 2147483647) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 30)) << (31 - 30)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 29)) << (31 - 29)) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 28)) << (31 - 28)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 27)) << (31 - 27)) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 26)) << (31 - 26)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 25)) << (31 - 25)) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 25;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 24)) << (31 - 24)) == value {
        *found = 7;
        return 7;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 23)) << (31 - 23)) == value {
        *found = 8;
        return 8;
    }
    tmp2 = tmp >> 23;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 22)) << (31 - 22)) == value {
        *found = 9;
        return 9;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 21)) << (31 - 21)) == value {
        *found = 10;
        return 10;
    }
    tmp2 = tmp >> 21;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (31 - 20)) == value {
        *found = 11;
        return 11;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 19)) << (31 - 19)) == value {
        *found = 12;
        return 12;
    }
    tmp2 = tmp >> 19;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 18)) << (31 - 18)) == value {
        *found = 13;
        return 13;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 17)) << (31 - 17)) == value {
        *found = 14;
        return 14;
    }
    tmp2 = tmp >> 17;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (31 - 16)) == value {
        *found = 15;
        return 15;
    }
    62
}

pub fn linsearch32_16(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    for i in 0..16 {
        if read_u32(input, i * 4) == value {
            *found = i as i32;
            return 0;
        }
    }
    64
}

pub fn linsearch1_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 1) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 1) & 1) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 2) & 1) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 3) & 1) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 4) & 1) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 5) & 1) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 6) & 1) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 7) & 1) == value {
        *found = 7;
        return 7;
    }
    1
}

pub fn linsearch2_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 3) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 2) & 3) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 4) & 3) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 6) & 3) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 8) & 3) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 10) & 3) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 12) & 3) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 14) & 3) == value {
        *found = 7;
        return 7;
    }
    2
}

pub fn linsearch3_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 7) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 3) & 7) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 6) & 7) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 9) & 7) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 12) & 7) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 15) & 7) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 18) & 7) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 21) & 7) == value {
        *found = 7;
        return 7;
    }
    3
}

pub fn linsearch4_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 15) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 4) & 15) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 8) & 15) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 12) & 15) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 16) & 15) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 20) & 15) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 24) & 15) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 28) & 15) == value {
        *found = 7;
        return 7;
    }
    4
}

pub fn linsearch5_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 31) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 5) & 31) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 10) & 31) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 15) & 31) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 20) & 31) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 25) & 31) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 3)) << (5 - 3)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 3) & 31) == value {
        *found = 7;
        return 7;
    }
    5
}

pub fn linsearch6_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 63) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 6) & 63) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 12) & 63) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 18) & 63) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 24) & 63) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (6 - 4)) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 4) & 63) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 10) & 63) == value {
        *found = 7;
        return 7;
    }
    6
}

pub fn linsearch7_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 127) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 7) & 127) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 14) & 127) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 21) & 127) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 3)) << (7 - 3)) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 3) & 127) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 10) & 127) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 17) & 127) == value {
        *found = 7;
        return 7;
    }
    7
}

pub fn linsearch8_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 255) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 8) & 255) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 16) & 255) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 24) & 255) == value {
        *found = 3;
        return 3;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 255) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 8) & 255) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 16) & 255) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 24) & 255) == value {
        *found = 7;
        return 7;
    }
    8
}

pub fn linsearch9_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 511) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 9) & 511) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 18) & 511) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (9 - 4)) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 4) & 511) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 13) & 511) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 22) & 511) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (9 - 8)) == value {
        *found = 7;
        return 7;
    }
    9
}

pub fn linsearch10_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 1023) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 10) & 1023) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 20) & 1023) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (10 - 8)) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 8) & 1023) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 18) & 1023) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (10 - 6)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 6) & 1023) == value {
        *found = 7;
        return 7;
    }
    10
}

pub fn linsearch11_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 2047) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 11) & 2047) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (11 - 1)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 1) & 2047) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 12) & 2047) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 23;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (11 - 2)) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 2) & 2047) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 13) & 2047) == value {
        *found = 7;
        return 7;
    }
    11
}

pub fn linsearch12_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 4095) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 12) & 4095) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (12 - 4)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 4) & 4095) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 16) & 4095) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (12 - 8)) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 8) & 4095) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 20) & 4095) == value {
        *found = 7;
        return 7;
    }
    12
}

pub fn linsearch13_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 8191) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 13) & 8191) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 7)) << (13 - 7)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 7) & 8191) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (13 - 1)) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 1) & 8191) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 14) & 8191) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (13 - 8)) == value {
        *found = 7;
        return 7;
    }
    13
}

pub fn linsearch14_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 16383) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 14) & 16383) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (14 - 10)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 10) & 16383) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (14 - 6)) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 6) & 16383) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (14 - 2)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 2) & 16383) == value {
        *found = 7;
        return 7;
    }
    14
}

pub fn linsearch15_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 32767) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 15) & 32767) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 13)) << (15 - 13)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 13) & 32767) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 11)) << (15 - 11)) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 11) & 32767) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 9)) << (15 - 9)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 9) & 32767) == value {
        *found = 7;
        return 7;
    }
    15
}

pub fn linsearch16_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 0;
        return 0;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 1;
        return 1;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 3;
        return 3;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 5;
        return 5;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 65535) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 16) & 65535) == value {
        *found = 7;
        return 7;
    }
    16
}

pub fn linsearch17_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 131071) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 17;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (17 - 2)) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 2) & 131071) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 19;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (17 - 4)) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 4) & 131071) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 21;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (17 - 6)) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 6) & 131071) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 23;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (17 - 8)) == value {
        *found = 7;
        return 7;
    }
    17
}

pub fn linsearch18_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 262143) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (18 - 4)) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 4) & 262143) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (18 - 8)) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 8) & 262143) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (18 - 12)) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 12) & 262143) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (18 - 16)) == value {
        *found = 7;
        return 7;
    }
    18
}

pub fn linsearch19_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 524287) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 19;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 6)) << (19 - 6)) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 6) & 524287) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 25;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (19 - 12)) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 12) & 524287) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 18)) << (19 - 18)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 5)) << (19 - 5)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 5) & 524287) == value {
        *found = 7;
        return 7;
    }
    19
}

pub fn linsearch20_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 1048575) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (20 - 8)) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 8) & 1048575) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (20 - 16)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (20 - 4)) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 4) & 1048575) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (20 - 12)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 12) & 1048575) == value {
        *found = 7;
        return 7;
    }
    20
}

pub fn linsearch21_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 2097151) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 21;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (21 - 10)) == value {
        *found = 1;
        return 1;
    }
    if ((tmp >> 10) & 2097151) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (21 - 20)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 9)) << (21 - 9)) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 9) & 2097151) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 19)) << (21 - 19)) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 19;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (21 - 8)) == value {
        *found = 7;
        return 7;
    }
    21
}

pub fn linsearch22_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 4194303) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (22 - 12)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (22 - 2)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 2) & 4194303) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (22 - 14)) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 14;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (22 - 4)) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 4) & 4194303) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (22 - 16)) == value {
        *found = 7;
        return 7;
    }
    22
}

pub fn linsearch23_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 8388607) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 23;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (23 - 14)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 14;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 5)) << (23 - 5)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 5) & 8388607) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 19)) << (23 - 19)) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 19;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 10)) << (23 - 10)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 10;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 1)) << (23 - 1)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 1) & 8388607) == value {
        *found = 7;
        return 7;
    }
    23
}

pub fn linsearch24_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 16777215) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (24 - 16)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (24 - 8)) == value {
        *found = 2;
        return 2;
    }
    if ((tmp >> 8) & 16777215) == value {
        *found = 3;
        return 3;
    }
    in_off += 4;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 16777215) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (24 - 16)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (24 - 8)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 8) & 16777215) == value {
        *found = 7;
        return 7;
    }
    24
}

pub fn linsearch25_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 33554431) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 25;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 18)) << (25 - 18)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 11)) << (25 - 11)) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 11;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (25 - 4)) == value {
        *found = 3;
        return 3;
    }
    if ((tmp >> 4) & 33554431) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 22)) << (25 - 22)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 15)) << (25 - 15)) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 15;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (25 - 8)) == value {
        *found = 7;
        return 7;
    }
    25
}

pub fn linsearch26_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 67108863) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (26 - 20)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (26 - 14)) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 14;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (26 - 8)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 8;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (26 - 2)) == value {
        *found = 4;
        return 4;
    }
    if ((tmp >> 2) & 67108863) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 22)) << (26 - 22)) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (26 - 16)) == value {
        *found = 7;
        return 7;
    }
    26
}

pub fn linsearch27_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 134217727) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 22)) << (27 - 22)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 17)) << (27 - 17)) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 17;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (27 - 12)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 7)) << (27 - 7)) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 7;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 2)) << (27 - 2)) == value {
        *found = 5;
        return 5;
    }
    if ((tmp >> 2) & 134217727) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 24)) << (27 - 24)) == value {
        *found = 7;
        return 7;
    }
    27
}

pub fn linsearch28_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 268435455) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 24)) << (28 - 24)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (28 - 20)) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (28 - 16)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 16;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 12)) << (28 - 12)) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 12;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (28 - 8)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 8;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 4)) << (28 - 4)) == value {
        *found = 6;
        return 6;
    }
    if ((tmp >> 4) & 268435455) == value {
        *found = 7;
        return 7;
    }
    28
}

pub fn linsearch29_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 536870911) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 26)) << (29 - 26)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 23)) << (29 - 23)) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 23;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (29 - 20)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 17)) << (29 - 17)) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 17;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 14)) << (29 - 14)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 14;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 11)) << (29 - 11)) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 11;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 8)) << (29 - 8)) == value {
        *found = 7;
        return 7;
    }
    29
}

pub fn linsearch30_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 1073741823) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 28)) << (30 - 28)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 26)) << (30 - 26)) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 24)) << (30 - 24)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 24;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 22)) << (30 - 22)) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 22;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 20)) << (30 - 20)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 20;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 18)) << (30 - 18)) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 18;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 16)) << (30 - 16)) == value {
        *found = 7;
        return 7;
    }
    30
}

pub fn linsearch31_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    let mut in_off: usize = 0;
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    tmp = read_u32(input, in_off);
    if ((tmp >> 0) & 2147483647) == value {
        *found = 0;
        return 0;
    }
    tmp2 = tmp >> 31;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 30)) << (31 - 30)) == value {
        *found = 1;
        return 1;
    }
    tmp2 = tmp >> 30;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 29)) << (31 - 29)) == value {
        *found = 2;
        return 2;
    }
    tmp2 = tmp >> 29;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 28)) << (31 - 28)) == value {
        *found = 3;
        return 3;
    }
    tmp2 = tmp >> 28;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 27)) << (31 - 27)) == value {
        *found = 4;
        return 4;
    }
    tmp2 = tmp >> 27;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 26)) << (31 - 26)) == value {
        *found = 5;
        return 5;
    }
    tmp2 = tmp >> 26;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 25)) << (31 - 25)) == value {
        *found = 6;
        return 6;
    }
    tmp2 = tmp >> 25;
    in_off += 4;
    tmp = read_u32(input, in_off);
    if (tmp2 | (tmp % (1u32 << 24)) << (31 - 24)) == value {
        *found = 7;
        return 7;
    }
    31
}

pub fn linsearch32_8(base: u32, input: &[u8], value: u32, found: &mut i32) -> u32 {
    let value = value.wrapping_sub(base);
    for i in 0..8 {
        if read_u32(input, i * 4) == value {
            *found = i as i32;
            return 0;
        }
    }
    32
}

pub fn linsearch1_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 1) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        if value == ((tmp >> 1) & 1) {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        if value == ((tmp >> 2) & 1) {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        if value == ((tmp >> 3) & 1) {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        if value == ((tmp >> 4) & 1) {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        if value == ((tmp >> 5) & 1) {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        if value == ((tmp >> 6) & 1) {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        if value == ((tmp >> 7) & 1) {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 1 + 7) / 8
}

pub fn linsearch2_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 3) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        if value == ((tmp >> 2) & 3) {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        if value == ((tmp >> 4) & 3) {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        if value == ((tmp >> 6) & 3) {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        if value == ((tmp >> 8) & 3) {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        if value == ((tmp >> 10) & 3) {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        if value == ((tmp >> 12) & 3) {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        if value == ((tmp >> 14) & 3) {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 2 + 7) / 8
}

pub fn linsearch3_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 7) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        if value == ((tmp >> 3) & 7) {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        if value == ((tmp >> 6) & 7) {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        if value == ((tmp >> 9) & 7) {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        if value == ((tmp >> 12) & 7) {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        if value == ((tmp >> 15) & 7) {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        if value == ((tmp >> 18) & 7) {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        if value == ((tmp >> 21) & 7) {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 3 + 7) / 8
}

pub fn linsearch4_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 15) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        if value == ((tmp >> 4) & 15) {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        if value == ((tmp >> 8) & 15) {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        if value == ((tmp >> 12) & 15) {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        if value == ((tmp >> 16) & 15) {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        if value == ((tmp >> 20) & 15) {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        if value == ((tmp >> 24) & 15) {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        if value == ((tmp >> 28) & 15) {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 4 + 7) / 8
}

pub fn linsearch5_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 31) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        if value == ((tmp >> 5) & 31) {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        if value == ((tmp >> 10) & 31) {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        if value == ((tmp >> 15) & 31) {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        if value == ((tmp >> 20) & 31) {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        if value == ((tmp >> 25) & 31) {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        tmp2 = tmp >> 30;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 3)) << (5 - 3)) == value {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        if value == ((tmp >> 3) & 31) {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 5 + 7) / 8
}

pub fn linsearch6_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 63) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        if value == ((tmp >> 6) & 63) {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        if value == ((tmp >> 12) & 63) {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        if value == ((tmp >> 18) & 63) {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        if value == ((tmp >> 24) & 63) {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        tmp2 = tmp >> 30;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 4)) << (6 - 4)) == value {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        if value == ((tmp >> 4) & 63) {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        if value == ((tmp >> 10) & 63) {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 6 + 7) / 8
}

pub fn linsearch7_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 127) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        if value == ((tmp >> 7) & 127) {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        if value == ((tmp >> 14) & 127) {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        if value == ((tmp >> 21) & 127) {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        tmp2 = tmp >> 28;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 3)) << (7 - 3)) == value {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        if value == ((tmp >> 3) & 127) {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        if value == ((tmp >> 10) & 127) {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        if value == ((tmp >> 17) & 127) {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 7 + 7) / 8
}

pub fn linsearch8_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 255) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        if value == ((tmp >> 8) & 255) {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        if value == ((tmp >> 16) & 255) {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        if value == ((tmp >> 24) & 255) {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 255) {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        if value == ((tmp >> 8) & 255) {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        if value == ((tmp >> 16) & 255) {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        if value == ((tmp >> 24) & 255) {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 8 + 7) / 8
}

pub fn linsearch9_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 511) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        if value == ((tmp >> 9) & 511) {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        if value == ((tmp >> 18) & 511) {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        tmp2 = tmp >> 27;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 4)) << (9 - 4)) == value {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        if value == ((tmp >> 4) & 511) {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        if value == ((tmp >> 13) & 511) {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        if value == ((tmp >> 22) & 511) {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        tmp2 = tmp >> 31;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 8)) << (9 - 8)) == value {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 9 + 7) / 8
}

pub fn linsearch10_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 1023) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        if value == ((tmp >> 10) & 1023) {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        if value == ((tmp >> 20) & 1023) {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        tmp2 = tmp >> 30;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 8)) << (10 - 8)) == value {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        if value == ((tmp >> 8) & 1023) {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        if value == ((tmp >> 18) & 1023) {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        tmp2 = tmp >> 28;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 6)) << (10 - 6)) == value {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        if value == ((tmp >> 6) & 1023) {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 10 + 7) / 8
}

pub fn linsearch11_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 2047) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        if value == ((tmp >> 11) & 2047) {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        tmp2 = tmp >> 22;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 1)) << (11 - 1)) == value {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        if value == ((tmp >> 1) & 2047) {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        if value == ((tmp >> 12) & 2047) {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        tmp2 = tmp >> 23;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 2)) << (11 - 2)) == value {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        if value == ((tmp >> 2) & 2047) {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        if value == ((tmp >> 13) & 2047) {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 11 + 7) / 8
}

pub fn linsearch12_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 4095) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        if value == ((tmp >> 12) & 4095) {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        tmp2 = tmp >> 24;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 4)) << (12 - 4)) == value {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        if value == ((tmp >> 4) & 4095) {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        if value == ((tmp >> 16) & 4095) {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        tmp2 = tmp >> 28;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 8)) << (12 - 8)) == value {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        if value == ((tmp >> 8) & 4095) {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        if value == ((tmp >> 20) & 4095) {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 12 + 7) / 8
}

pub fn linsearch13_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 8191) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        if value == ((tmp >> 13) & 8191) {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        tmp2 = tmp >> 26;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 7)) << (13 - 7)) == value {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        if value == ((tmp >> 7) & 8191) {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        tmp2 = tmp >> 20;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 1)) << (13 - 1)) == value {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        if value == ((tmp >> 1) & 8191) {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        if value == ((tmp >> 14) & 8191) {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        tmp2 = tmp >> 27;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 8)) << (13 - 8)) == value {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 13 + 7) / 8
}

pub fn linsearch14_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 16383) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        if value == ((tmp >> 14) & 16383) {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        tmp2 = tmp >> 28;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 10)) << (14 - 10)) == value {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        if value == ((tmp >> 10) & 16383) {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        tmp2 = tmp >> 24;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 6)) << (14 - 6)) == value {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        if value == ((tmp >> 6) & 16383) {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        tmp2 = tmp >> 20;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 2)) << (14 - 2)) == value {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        if value == ((tmp >> 2) & 16383) {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 14 + 7) / 8
}

pub fn linsearch15_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 32767) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        if value == ((tmp >> 15) & 32767) {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        tmp2 = tmp >> 30;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 13)) << (15 - 13)) == value {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        if value == ((tmp >> 13) & 32767) {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        tmp2 = tmp >> 28;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 11)) << (15 - 11)) == value {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        if value == ((tmp >> 11) & 32767) {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        tmp2 = tmp >> 26;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 9)) << (15 - 9)) == value {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        if value == ((tmp >> 9) & 32767) {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 15 + 7) / 8
}

pub fn linsearch16_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 65535) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        if value == ((tmp >> 16) & 65535) {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 65535) {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        if value == ((tmp >> 16) & 65535) {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 65535) {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        if value == ((tmp >> 16) & 65535) {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 65535) {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        if value == ((tmp >> 16) & 65535) {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 16 + 7) / 8
}

pub fn linsearch17_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 131071) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        tmp2 = tmp >> 17;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 2)) << (17 - 2)) == value {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        if value == ((tmp >> 2) & 131071) {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        tmp2 = tmp >> 19;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 4)) << (17 - 4)) == value {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        if value == ((tmp >> 4) & 131071) {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        tmp2 = tmp >> 21;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 6)) << (17 - 6)) == value {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        if value == ((tmp >> 6) & 131071) {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        tmp2 = tmp >> 23;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 8)) << (17 - 8)) == value {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 17 + 7) / 8
}

pub fn linsearch18_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 262143) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        tmp2 = tmp >> 18;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 4)) << (18 - 4)) == value {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        if value == ((tmp >> 4) & 262143) {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        tmp2 = tmp >> 22;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 8)) << (18 - 8)) == value {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        if value == ((tmp >> 8) & 262143) {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        tmp2 = tmp >> 26;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 12)) << (18 - 12)) == value {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        if value == ((tmp >> 12) & 262143) {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        tmp2 = tmp >> 30;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 16)) << (18 - 16)) == value {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 18 + 7) / 8
}

pub fn linsearch19_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 524287) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        tmp2 = tmp >> 19;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 6)) << (19 - 6)) == value {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        if value == ((tmp >> 6) & 524287) {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        tmp2 = tmp >> 25;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 12)) << (19 - 12)) == value {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        if value == ((tmp >> 12) & 524287) {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        tmp2 = tmp >> 31;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 18)) << (19 - 18)) == value {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        tmp2 = tmp >> 18;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 5)) << (19 - 5)) == value {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        if value == ((tmp >> 5) & 524287) {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 19 + 7) / 8
}

pub fn linsearch20_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 1048575) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        tmp2 = tmp >> 20;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 8)) << (20 - 8)) == value {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        if value == ((tmp >> 8) & 1048575) {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        tmp2 = tmp >> 28;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 16)) << (20 - 16)) == value {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        tmp2 = tmp >> 16;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 4)) << (20 - 4)) == value {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        if value == ((tmp >> 4) & 1048575) {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        tmp2 = tmp >> 24;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 12)) << (20 - 12)) == value {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        if value == ((tmp >> 12) & 1048575) {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 20 + 7) / 8
}

pub fn linsearch21_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 2097151) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        tmp2 = tmp >> 21;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 10)) << (21 - 10)) == value {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        if value == ((tmp >> 10) & 2097151) {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        tmp2 = tmp >> 31;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 20)) << (21 - 20)) == value {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        tmp2 = tmp >> 20;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 9)) << (21 - 9)) == value {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        if value == ((tmp >> 9) & 2097151) {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        tmp2 = tmp >> 30;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 19)) << (21 - 19)) == value {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        tmp2 = tmp >> 19;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 8)) << (21 - 8)) == value {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 21 + 7) / 8
}

pub fn linsearch22_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 4194303) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        tmp2 = tmp >> 22;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 12)) << (22 - 12)) == value {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        tmp2 = tmp >> 12;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 2)) << (22 - 2)) == value {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        if value == ((tmp >> 2) & 4194303) {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        tmp2 = tmp >> 24;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 14)) << (22 - 14)) == value {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        tmp2 = tmp >> 14;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 4)) << (22 - 4)) == value {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        if value == ((tmp >> 4) & 4194303) {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        tmp2 = tmp >> 26;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 16)) << (22 - 16)) == value {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 22 + 7) / 8
}

pub fn linsearch23_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 8388607) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        tmp2 = tmp >> 23;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 14)) << (23 - 14)) == value {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        tmp2 = tmp >> 14;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 5)) << (23 - 5)) == value {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        if value == ((tmp >> 5) & 8388607) {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        tmp2 = tmp >> 28;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 19)) << (23 - 19)) == value {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        tmp2 = tmp >> 19;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 10)) << (23 - 10)) == value {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        tmp2 = tmp >> 10;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 1)) << (23 - 1)) == value {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        if value == ((tmp >> 1) & 8388607) {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 23 + 7) / 8
}

pub fn linsearch24_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 16777215) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        tmp2 = tmp >> 24;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 16)) << (24 - 16)) == value {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        tmp2 = tmp >> 16;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 8)) << (24 - 8)) == value {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        if value == ((tmp >> 8) & 16777215) {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 16777215) {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        tmp2 = tmp >> 24;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 16)) << (24 - 16)) == value {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        tmp2 = tmp >> 16;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 8)) << (24 - 8)) == value {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        if value == ((tmp >> 8) & 16777215) {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 24 + 7) / 8
}

pub fn linsearch25_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 33554431) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        tmp2 = tmp >> 25;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 18)) << (25 - 18)) == value {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        tmp2 = tmp >> 18;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 11)) << (25 - 11)) == value {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        tmp2 = tmp >> 11;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 4)) << (25 - 4)) == value {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        if value == ((tmp >> 4) & 33554431) {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        tmp2 = tmp >> 29;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 22)) << (25 - 22)) == value {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        tmp2 = tmp >> 22;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 15)) << (25 - 15)) == value {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        tmp2 = tmp >> 15;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 8)) << (25 - 8)) == value {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 25 + 7) / 8
}

pub fn linsearch26_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 67108863) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        tmp2 = tmp >> 26;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 20)) << (26 - 20)) == value {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        tmp2 = tmp >> 20;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 14)) << (26 - 14)) == value {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        tmp2 = tmp >> 14;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 8)) << (26 - 8)) == value {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        tmp2 = tmp >> 8;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 2)) << (26 - 2)) == value {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        if value == ((tmp >> 2) & 67108863) {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        tmp2 = tmp >> 28;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 22)) << (26 - 22)) == value {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        tmp2 = tmp >> 22;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 16)) << (26 - 16)) == value {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 26 + 7) / 8
}

pub fn linsearch27_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 134217727) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        tmp2 = tmp >> 27;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 22)) << (27 - 22)) == value {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        tmp2 = tmp >> 22;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 17)) << (27 - 17)) == value {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        tmp2 = tmp >> 17;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 12)) << (27 - 12)) == value {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        tmp2 = tmp >> 12;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 7)) << (27 - 7)) == value {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        tmp2 = tmp >> 7;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 2)) << (27 - 2)) == value {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        if value == ((tmp >> 2) & 134217727) {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        tmp2 = tmp >> 29;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 24)) << (27 - 24)) == value {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 27 + 7) / 8
}

pub fn linsearch28_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 268435455) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        tmp2 = tmp >> 28;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 24)) << (28 - 24)) == value {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        tmp2 = tmp >> 24;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 20)) << (28 - 20)) == value {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        tmp2 = tmp >> 20;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 16)) << (28 - 16)) == value {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        tmp2 = tmp >> 16;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 12)) << (28 - 12)) == value {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        tmp2 = tmp >> 12;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 8)) << (28 - 8)) == value {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        tmp2 = tmp >> 8;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 4)) << (28 - 4)) == value {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        if value == ((tmp >> 4) & 268435455) {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 28 + 7) / 8
}

pub fn linsearch29_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 536870911) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        tmp2 = tmp >> 29;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 26)) << (29 - 26)) == value {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        tmp2 = tmp >> 26;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 23)) << (29 - 23)) == value {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        tmp2 = tmp >> 23;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 20)) << (29 - 20)) == value {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        tmp2 = tmp >> 20;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 17)) << (29 - 17)) == value {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        tmp2 = tmp >> 17;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 14)) << (29 - 14)) == value {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        tmp2 = tmp >> 14;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 11)) << (29 - 11)) == value {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        tmp2 = tmp >> 11;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 8)) << (29 - 8)) == value {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 29 + 7) / 8
}

pub fn linsearch30_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 1073741823) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        tmp2 = tmp >> 30;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 28)) << (30 - 28)) == value {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        tmp2 = tmp >> 28;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 26)) << (30 - 26)) == value {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        tmp2 = tmp >> 26;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 24)) << (30 - 24)) == value {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        tmp2 = tmp >> 24;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 22)) << (30 - 22)) == value {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        tmp2 = tmp >> 22;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 20)) << (30 - 20)) == value {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        tmp2 = tmp >> 20;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 18)) << (30 - 18)) == value {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        tmp2 = tmp >> 18;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 16)) << (30 - 16)) == value {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 30 + 7) / 8
}

pub fn linsearch31_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 2147483647) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        tmp2 = tmp >> 31;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 30)) << (31 - 30)) == value {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        tmp2 = tmp >> 30;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 29)) << (31 - 29)) == value {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        tmp2 = tmp >> 29;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 28)) << (31 - 28)) == value {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        tmp2 = tmp >> 28;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 27)) << (31 - 27)) == value {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        tmp2 = tmp >> 27;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 26)) << (31 - 26)) == value {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        tmp2 = tmp >> 26;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 25)) << (31 - 25)) == value {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        tmp2 = tmp >> 25;
        in_off += 4;
        tmp = read_u32(input, in_off);
        if (tmp2 | (tmp % (1u32 << 24)) << (31 - 24)) == value {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 31 + 7) / 8
}

pub fn linsearch32_x(base: u32, input: &[u8], length: u32, value: u32, found: &mut i32) -> u32 {
    if length == 0 { return 0; }
    let value = value.wrapping_sub(base);
    let mut tmp: u32 = 0;
    let mut tmp2: u32;
    let mut in_off: usize = 0;
    'bail: loop {
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 4294967295) {
            *found = 0;
            return 0;
        }
        if length == 1 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 4294967295) {
            *found = 1;
            return 1;
        }
        if length == 2 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 4294967295) {
            *found = 2;
            return 2;
        }
        if length == 3 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 4294967295) {
            *found = 3;
            return 3;
        }
        if length == 4 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 4294967295) {
            *found = 4;
            return 4;
        }
        if length == 5 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 4294967295) {
            *found = 5;
            return 5;
        }
        if length == 6 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 4294967295) {
            *found = 6;
            return 6;
        }
        if length == 7 { break 'bail; }
        in_off += 4;
        tmp = read_u32(input, in_off);
        if value == ((tmp >> 0) & 4294967295) {
            *found = 7;
            return 7;
        }
        if length == 8 { break 'bail; }
        break 'bail;
    }
    (length * 32 + 7) / 8
}

pub type PackFn = fn(u32, &[u32], &mut [u8]) -> u32;
pub type UnpackFn = fn(u32, &[u8], &mut [u32]) -> u32;
pub type PackXFn = fn(u32, &[u32], &mut [u8], u32) -> u32;
pub type UnpackXFn = fn(u32, &[u8], &mut [u32], u32) -> u32;
pub type LinsearchFn = fn(u32, &[u8], u32, &mut i32) -> u32;
pub type LinsearchXFn = fn(u32, &[u8], u32, u32, &mut i32) -> u32;

pub static FOR_PACK32: [PackFn; 33] = [
    pack0_32,
    pack1_32,
    pack2_32,
    pack3_32,
    pack4_32,
    pack5_32,
    pack6_32,
    pack7_32,
    pack8_32,
    pack9_32,
    pack10_32,
    pack11_32,
    pack12_32,
    pack13_32,
    pack14_32,
    pack15_32,
    pack16_32,
    pack17_32,
    pack18_32,
    pack19_32,
    pack20_32,
    pack21_32,
    pack22_32,
    pack23_32,
    pack24_32,
    pack25_32,
    pack26_32,
    pack27_32,
    pack28_32,
    pack29_32,
    pack30_32,
    pack31_32,
    pack32_32,
];

pub static FOR_UNPACK32: [UnpackFn; 33] = [
    unpack0_32,
    unpack1_32,
    unpack2_32,
    unpack3_32,
    unpack4_32,
    unpack5_32,
    unpack6_32,
    unpack7_32,
    unpack8_32,
    unpack9_32,
    unpack10_32,
    unpack11_32,
    unpack12_32,
    unpack13_32,
    unpack14_32,
    unpack15_32,
    unpack16_32,
    unpack17_32,
    unpack18_32,
    unpack19_32,
    unpack20_32,
    unpack21_32,
    unpack22_32,
    unpack23_32,
    unpack24_32,
    unpack25_32,
    unpack26_32,
    unpack27_32,
    unpack28_32,
    unpack29_32,
    unpack30_32,
    unpack31_32,
    unpack32_32,
];

pub static FOR_PACK16: [PackFn; 33] = [
    pack0_16,
    pack1_16,
    pack2_16,
    pack3_16,
    pack4_16,
    pack5_16,
    pack6_16,
    pack7_16,
    pack8_16,
    pack9_16,
    pack10_16,
    pack11_16,
    pack12_16,
    pack13_16,
    pack14_16,
    pack15_16,
    pack16_16,
    pack17_16,
    pack18_16,
    pack19_16,
    pack20_16,
    pack21_16,
    pack22_16,
    pack23_16,
    pack24_16,
    pack25_16,
    pack26_16,
    pack27_16,
    pack28_16,
    pack29_16,
    pack30_16,
    pack31_16,
    pack32_16,
];

pub static FOR_UNPACK16: [UnpackFn; 33] = [
    unpack0_16,
    unpack1_16,
    unpack2_16,
    unpack3_16,
    unpack4_16,
    unpack5_16,
    unpack6_16,
    unpack7_16,
    unpack8_16,
    unpack9_16,
    unpack10_16,
    unpack11_16,
    unpack12_16,
    unpack13_16,
    unpack14_16,
    unpack15_16,
    unpack16_16,
    unpack17_16,
    unpack18_16,
    unpack19_16,
    unpack20_16,
    unpack21_16,
    unpack22_16,
    unpack23_16,
    unpack24_16,
    unpack25_16,
    unpack26_16,
    unpack27_16,
    unpack28_16,
    unpack29_16,
    unpack30_16,
    unpack31_16,
    unpack32_16,
];

pub static FOR_PACK8: [PackFn; 33] = [
    pack0_8,
    pack1_8,
    pack2_8,
    pack3_8,
    pack4_8,
    pack5_8,
    pack6_8,
    pack7_8,
    pack8_8,
    pack9_8,
    pack10_8,
    pack11_8,
    pack12_8,
    pack13_8,
    pack14_8,
    pack15_8,
    pack16_8,
    pack17_8,
    pack18_8,
    pack19_8,
    pack20_8,
    pack21_8,
    pack22_8,
    pack23_8,
    pack24_8,
    pack25_8,
    pack26_8,
    pack27_8,
    pack28_8,
    pack29_8,
    pack30_8,
    pack31_8,
    pack32_8,
];

pub static FOR_UNPACK8: [UnpackFn; 33] = [
    unpack0_8,
    unpack1_8,
    unpack2_8,
    unpack3_8,
    unpack4_8,
    unpack5_8,
    unpack6_8,
    unpack7_8,
    unpack8_8,
    unpack9_8,
    unpack10_8,
    unpack11_8,
    unpack12_8,
    unpack13_8,
    unpack14_8,
    unpack15_8,
    unpack16_8,
    unpack17_8,
    unpack18_8,
    unpack19_8,
    unpack20_8,
    unpack21_8,
    unpack22_8,
    unpack23_8,
    unpack24_8,
    unpack25_8,
    unpack26_8,
    unpack27_8,
    unpack28_8,
    unpack29_8,
    unpack30_8,
    unpack31_8,
    unpack32_8,
];

pub static FOR_PACKX: [PackXFn; 33] = [
    pack0_x,
    pack1_x,
    pack2_x,
    pack3_x,
    pack4_x,
    pack5_x,
    pack6_x,
    pack7_x,
    pack8_x,
    pack9_x,
    pack10_x,
    pack11_x,
    pack12_x,
    pack13_x,
    pack14_x,
    pack15_x,
    pack16_x,
    pack17_x,
    pack18_x,
    pack19_x,
    pack20_x,
    pack21_x,
    pack22_x,
    pack23_x,
    pack24_x,
    pack25_x,
    pack26_x,
    pack27_x,
    pack28_x,
    pack29_x,
    pack30_x,
    pack31_x,
    pack32_x,
];

pub static FOR_UNPACKX: [UnpackXFn; 33] = [
    unpack0_x,
    unpack1_x,
    unpack2_x,
    unpack3_x,
    unpack4_x,
    unpack5_x,
    unpack6_x,
    unpack7_x,
    unpack8_x,
    unpack9_x,
    unpack10_x,
    unpack11_x,
    unpack12_x,
    unpack13_x,
    unpack14_x,
    unpack15_x,
    unpack16_x,
    unpack17_x,
    unpack18_x,
    unpack19_x,
    unpack20_x,
    unpack21_x,
    unpack22_x,
    unpack23_x,
    unpack24_x,
    unpack25_x,
    unpack26_x,
    unpack27_x,
    unpack28_x,
    unpack29_x,
    unpack30_x,
    unpack31_x,
    unpack32_x,
];

pub static FOR_LINSEARCH32: [LinsearchFn; 33] = [
    linsearch0_32,
    linsearch1_32,
    linsearch2_32,
    linsearch3_32,
    linsearch4_32,
    linsearch5_32,
    linsearch6_32,
    linsearch7_32,
    linsearch8_32,
    linsearch9_32,
    linsearch10_32,
    linsearch11_32,
    linsearch12_32,
    linsearch13_32,
    linsearch14_32,
    linsearch15_32,
    linsearch16_32,
    linsearch17_32,
    linsearch18_32,
    linsearch19_32,
    linsearch20_32,
    linsearch21_32,
    linsearch22_32,
    linsearch23_32,
    linsearch24_32,
    linsearch25_32,
    linsearch26_32,
    linsearch27_32,
    linsearch28_32,
    linsearch29_32,
    linsearch30_32,
    linsearch31_32,
    linsearch32_32,
];

pub static FOR_LINSEARCH16: [LinsearchFn; 33] = [
    linsearch0_16,
    linsearch1_16,
    linsearch2_16,
    linsearch3_16,
    linsearch4_16,
    linsearch5_16,
    linsearch6_16,
    linsearch7_16,
    linsearch8_16,
    linsearch9_16,
    linsearch10_16,
    linsearch11_16,
    linsearch12_16,
    linsearch13_16,
    linsearch14_16,
    linsearch15_16,
    linsearch16_16,
    linsearch17_16,
    linsearch18_16,
    linsearch19_16,
    linsearch20_16,
    linsearch21_16,
    linsearch22_16,
    linsearch23_16,
    linsearch24_16,
    linsearch25_16,
    linsearch26_16,
    linsearch27_16,
    linsearch28_16,
    linsearch29_16,
    linsearch30_16,
    linsearch31_16,
    linsearch32_16,
];

pub static FOR_LINSEARCH8: [LinsearchFn; 33] = [
    linsearch0_8,
    linsearch1_8,
    linsearch2_8,
    linsearch3_8,
    linsearch4_8,
    linsearch5_8,
    linsearch6_8,
    linsearch7_8,
    linsearch8_8,
    linsearch9_8,
    linsearch10_8,
    linsearch11_8,
    linsearch12_8,
    linsearch13_8,
    linsearch14_8,
    linsearch15_8,
    linsearch16_8,
    linsearch17_8,
    linsearch18_8,
    linsearch19_8,
    linsearch20_8,
    linsearch21_8,
    linsearch22_8,
    linsearch23_8,
    linsearch24_8,
    linsearch25_8,
    linsearch26_8,
    linsearch27_8,
    linsearch28_8,
    linsearch29_8,
    linsearch30_8,
    linsearch31_8,
    linsearch32_8,
];

pub static FOR_LINSEARCHX: [LinsearchXFn; 33] = [
    linsearch0_x,
    linsearch1_x,
    linsearch2_x,
    linsearch3_x,
    linsearch4_x,
    linsearch5_x,
    linsearch6_x,
    linsearch7_x,
    linsearch8_x,
    linsearch9_x,
    linsearch10_x,
    linsearch11_x,
    linsearch12_x,
    linsearch13_x,
    linsearch14_x,
    linsearch15_x,
    linsearch16_x,
    linsearch17_x,
    linsearch18_x,
    linsearch19_x,
    linsearch20_x,
    linsearch21_x,
    linsearch22_x,
    linsearch23_x,
    linsearch24_x,
    linsearch25_x,
    linsearch26_x,
    linsearch27_x,
    linsearch28_x,
    linsearch29_x,
    linsearch30_x,
    linsearch31_x,
    linsearch32_x,
];
