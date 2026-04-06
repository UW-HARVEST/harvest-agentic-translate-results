use crate::types::{CHAR_MAP, LSF};
const U40_9: u64 = 40_u64.pow(9);
const U40_9_8: u64 = U40_9 + 40_u64.pow(8);

pub fn decode_callsign_value(outp: &mut [u8], inp: u64) {
    let mut encoded = inp;
    let mut start: usize = 0;

    if encoded >= U40_9 {
        if encoded == 0xFFFFFFFFFFFF {
            outp[..4].copy_from_slice(b"@ALL");
            outp[4] = 0;
            return;
        } else if encoded <= U40_9_8 {
            start = 1;
            encoded -= U40_9;
            outp[0] = b'#';
        } else {
            return;
        }
    }

    let char_map = CHAR_MAP.as_bytes();
    let mut i = start;
    while encoded > 0 {
        outp[i] = char_map[(encoded % 40) as usize];
        encoded /= 40;
        i += 1;
    }
    outp[i] = 0;
}

pub fn decode_callsign_bytes(outp: &mut [u8], inp: &[u8; 6]) {
    let mut encoded: u64 = 0;
    for i in 0..6 {
        encoded |= (inp[5 - i] as u64) << (8 * i);
    }
    decode_callsign_value(outp, encoded);
}

pub fn encode_callsign_value(inp: &[u8]) -> Option<u64> {
    let len = inp.iter().position(|&b| b == 0).unwrap_or(inp.len());
    if len > 9 {
        return None;
    }

    let s = &inp[..len];

    if s == b"@ALL" {
        return Some(0xFFFFFFFFFFFF);
    }

    let char_map = CHAR_MAP.as_bytes();
    let mut tmp: u64 = 0;
    let mut start: usize = 0;

    if !s.is_empty() && s[0] == b'#' {
        start = 1;
    }

    for i in (start..len).rev() {
        for j in 0..40 {
            if s[i] == char_map[j] {
                tmp = tmp * 40 + j as u64;
                break;
            }
        }
    }

    if start > 0 {
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
    let mut crc: u32 = 0xFFFF;
    for &byte in input {
        crc ^= (byte as u32) << 8;
        for _ in 0..8 {
            crc <<= 1;
            if crc & 0x10000 != 0 {
                crc = (crc ^ M17_CRC_POLY as u32) & 0xFFFF;
            }
        }
    }
    (crc & 0xFFFF) as u16
}

pub fn lsf_crc(input: &LSF) -> u16 {
    let mut d = [0u8; 28];
    d[0..6].copy_from_slice(&input.dst);
    d[6..12].copy_from_slice(&input.src);
    d[12..14].copy_from_slice(&input.type_field);
    d[14..28].copy_from_slice(&input.meta);
    crc_m17(&d)
}

pub fn extract_lich(outp: &mut [u8; 6], cnt: u8, inp: &LSF) {
    match cnt {
        0 => {
            outp[0] = inp.dst[0]; outp[1] = inp.dst[1]; outp[2] = inp.dst[2];
            outp[3] = inp.dst[3]; outp[4] = inp.dst[4];
        }
        1 => {
            outp[0] = inp.dst[5]; outp[1] = inp.src[0]; outp[2] = inp.src[1];
            outp[3] = inp.src[2]; outp[4] = inp.src[3];
        }
        2 => {
            outp[0] = inp.src[4]; outp[1] = inp.src[5]; outp[2] = inp.type_field[0];
            outp[3] = inp.type_field[1]; outp[4] = inp.meta[0];
        }
        3 => {
            outp[0] = inp.meta[1]; outp[1] = inp.meta[2]; outp[2] = inp.meta[3];
            outp[3] = inp.meta[4]; outp[4] = inp.meta[5];
        }
        4 => {
            outp[0] = inp.meta[6]; outp[1] = inp.meta[7]; outp[2] = inp.meta[8];
            outp[3] = inp.meta[9]; outp[4] = inp.meta[10];
        }
        5 => {
            outp[0] = inp.meta[11]; outp[1] = inp.meta[12]; outp[2] = inp.meta[13];
            outp[3] = inp.crc[0]; outp[4] = inp.crc[1];
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
