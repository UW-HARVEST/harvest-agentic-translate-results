use crate::types::{CHAR_MAP, LSF};
const U40_9: u64 = 40_u64.pow(9);
const U40_9_8: u64 = U40_9 + 40_u64.pow(8);
pub fn decode_callsign_value(outp: &mut [u8], inp: u64) {
    let mut encoded = inp;
    let mut start = 0usize;

    if encoded >= U40_9 {
        if encoded == 0xFFFF_FFFF_FFFF {
            write_c_string(outp, b"@ALL");
            return;
        } else if encoded <= U40_9_8 {
            start = 1;
            encoded -= U40_9;
            if !outp.is_empty() {
                outp[0] = b'#';
            }
        } else {
            return;
        }
    }

    let map = CHAR_MAP.as_bytes();
    let mut i = start;
    while encoded > 0 {
        if i < outp.len() {
            outp[i] = map[(encoded % 40) as usize];
        }
        encoded /= 40;
        i += 1;
    }
    if i < outp.len() {
        outp[i] = 0;
    }
}
pub fn decode_callsign_bytes(outp: &mut [u8], inp: &[u8; 6]) {
    let mut encoded = 0u64;
    for i in 0..6 {
        encoded |= u64::from(inp[5 - i]) << (8 * i);
    }
    decode_callsign_value(outp, encoded);
}
pub fn encode_callsign_value(inp: &[u8]) -> Option<u64> {
    if inp.len() > 9 {
        return None;
    }

    if inp == b"@ALL" {
        return Some(0xFFFF_FFFF_FFFF);
    }

    let char_map = CHAR_MAP.as_bytes();
    let mut tmp = 0u64;
    let start = usize::from(inp.first() == Some(&b'#'));

    for &ch in inp[start..].iter().rev() {
        for (j, &mapped) in char_map.iter().enumerate() {
            if ch == mapped {
                tmp = tmp * 40 + j as u64;
                break;
            }
        }
    }

    if start != 0 {
        tmp += U40_9;
    }

    Some(tmp)
}
pub fn encode_callsign_bytes(inp: &[u8]) -> Option<[u8; 6]> {
    let tmp = encode_callsign_value(inp)?;
    let mut out = [0u8; 6];
    for i in 0..6 {
        out[5 - i] = ((tmp >> (8 * i)) & 0xFF) as u8;
    }
    Some(out)
}
const M17_CRC_POLY: u16 = 0x5935;
pub fn crc_m17(input: &[u8]) -> u16 {
    let mut crc = 0xFFFFu32;

    for &byte in input {
        crc ^= u32::from(byte) << 8;
        for _ in 0..8 {
            crc <<= 1;
            if (crc & 0x10000) != 0 {
                crc = (crc ^ u32::from(M17_CRC_POLY)) & 0xFFFF;
            }
        }
    }

    (crc & 0xFFFF) as u16
}
pub fn lsf_crc(input: &LSF) -> u16 {
    let mut data = [0u8; 28];
    data[0..6].copy_from_slice(&input.dst);
    data[6..12].copy_from_slice(&input.src);
    data[12..14].copy_from_slice(&input.type_field);
    data[14..28].copy_from_slice(&input.meta);
    crc_m17(&data)
}
pub fn extract_lich(outp: &mut [u8; 6], cnt: u8, inp: &LSF) {
    match cnt {
        0 => {
            outp[0..5].copy_from_slice(&inp.dst[0..5]);
        }
        1 => {
            outp[0] = inp.dst[5];
            outp[1..5].copy_from_slice(&inp.src[0..4]);
        }
        2 => {
            outp[0] = inp.src[4];
            outp[1] = inp.src[5];
            outp[2] = inp.type_field[0];
            outp[3] = inp.type_field[1];
            outp[4] = inp.meta[0];
        }
        3 => {
            outp[0..5].copy_from_slice(&inp.meta[1..6]);
        }
        4 => {
            outp[0..5].copy_from_slice(&inp.meta[6..11]);
        }
        5 => {
            outp[0] = inp.meta[11];
            outp[1] = inp.meta[12];
            outp[2] = inp.meta[13];
            outp[3] = inp.crc[0];
            outp[4] = inp.crc[1];
        }
        _ => {}
    }

    outp[5] = cnt << 5;
}
pub fn unpack_lich(out: &mut [u8], input: &[u8; 12]) {
    for i in 0..12 {
        for j in 0..8 {
            out[i * 8 + j] = (input[i] >> (7 - j)) & 1;
        }
    }
}

fn write_c_string(out: &mut [u8], bytes: &[u8]) {
    for (dst, src) in out.iter_mut().zip(bytes.iter().copied()) {
        *dst = src;
    }
    if bytes.len() < out.len() {
        out[bytes.len()] = 0;
    }
}
