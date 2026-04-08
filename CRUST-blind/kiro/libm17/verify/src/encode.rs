use crate::types::LSF;
// Puncture patterns as constant arrays
pub const PUNCTURE_PATTERN_1: [u8; 61] = [
    1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1,
    1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1,
];
pub const PUNCTURE_PATTERN_2: [u8; 12] = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0];
pub const PUNCTURE_PATTERN_3: [u8; 8] = [1, 1, 1, 1, 1, 1, 1, 0];
pub const SYMBOL_MAP: [i8; 4] = [1, 3, -1, -3];
pub const SYMBOL_LIST: [i8; 4] = [-3, -1, 1, 3];
pub const EOT_SYMBOLS: [f32; 8] = [3.0, 3.0, 3.0, 3.0, 3.0, 3.0, -3.0, 3.0];

fn conv_encode_core(out: &mut [u8], ud: &[u8], data_bits: usize, punct: &[u8]) {
    let pp_len = punct.len();
    let mut p: usize = 0;
    let mut pb: usize = 0;

    for i in 0..data_bits + 4 {
        let g1 = (ud[i + 4] + ud[i + 1] + ud[i + 0]) % 2;
        let g2 = (ud[i + 4] + ud[i + 3] + ud[i + 2] + ud[i + 0]) % 2;

        if punct[p] != 0 {
            out[pb] = g1;
            pb += 1;
        }
        p = (p + 1) % pp_len;

        if punct[p] != 0 {
            out[pb] = g2;
            pb += 1;
        }
        p = (p + 1) % pp_len;
    }
}

pub fn conv_encode_stream_frame(out: &mut [u8], input: &[u8], fn_num: u16) {
    let mut ud = [0u8; 144 + 4 + 4];

    // unpack frame number
    for i in 0..16usize {
        ud[4 + i] = ((fn_num >> (15 - i)) & 1) as u8;
    }

    // unpack data (16 bytes)
    for i in 0..16usize {
        for j in 0..8usize {
            ud[4 + 16 + i * 8 + j] = (input[i] >> (7 - j)) & 1;
        }
    }

    conv_encode_core(out, &ud, 144, &PUNCTURE_PATTERN_2);
}

pub fn conv_encode_packet_frame(out: &mut [u8], input: &[u8]) {
    let mut ud = [0u8; 206 + 4 + 4];

    // unpack data (26 bytes, but last byte only 6 bits)
    for i in 0..26usize {
        for j in 0..8usize {
            if i <= 24 || j <= 5 {
                ud[4 + i * 8 + j] = (input[i] >> (7 - j)) & 1;
            }
        }
    }

    conv_encode_core(out, &ud, 206, &PUNCTURE_PATTERN_3);
}

pub fn conv_encode_lsf(out: &mut [u8], input: &LSF) {
    let mut ud = [0u8; 240 + 4 + 4];

    // unpack DST
    for i in 0..8usize {
        for k in 0..6usize {
            ud[4 + i + k * 8] = (input.dst[k] >> (7 - i)) & 1;
        }
    }

    // unpack SRC
    for i in 0..8usize {
        for k in 0..6usize {
            ud[4 + 48 + i + k * 8] = (input.src[k] >> (7 - i)) & 1;
        }
    }

    // unpack TYPE
    for i in 0..8usize {
        ud[4 + i + 96] = (input.type_field[0] >> (7 - i)) & 1;
        ud[4 + i + 104] = (input.type_field[1] >> (7 - i)) & 1;
    }

    // unpack META
    for i in 0..8usize {
        for k in 0..14usize {
            ud[4 + 112 + i + k * 8] = (input.meta[k] >> (7 - i)) & 1;
        }
    }

    // unpack CRC
    for i in 0..8usize {
        ud[4 + i + 224] = (input.crc[0] >> (7 - i)) & 1;
        ud[4 + i + 232] = (input.crc[1] >> (7 - i)) & 1;
    }

    conv_encode_core(out, &ud, 240, &PUNCTURE_PATTERN_1);
}
