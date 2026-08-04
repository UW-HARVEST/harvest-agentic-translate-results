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
pub fn conv_encode_stream_frame(out: &mut [u8], input: &[u8], fn_num: u16) {
    let mut ud = [0u8; 144 + 4 + 4];

    for i in 0..16 {
        ud[4 + i] = ((fn_num >> (15 - i)) & 1) as u8;
    }

    for i in 0..16 {
        for j in 0..8 {
            ud[4 + 16 + i * 8 + j] = (input[i] >> (7 - j)) & 1;
        }
    }

    encode_convolutional(out, &ud, 144 + 4, &PUNCTURE_PATTERN_2);
}
pub fn conv_encode_packet_frame(out: &mut [u8], input: &[u8]) {
    let mut ud = [0u8; 206 + 4 + 4];

    for i in 0..26 {
        for j in 0..8 {
            if i <= 24 || j <= 5 {
                ud[4 + i * 8 + j] = (input[i] >> (7 - j)) & 1;
            }
        }
    }

    encode_convolutional(out, &ud, 206 + 4, &PUNCTURE_PATTERN_3);
}
pub fn conv_encode_lsf(out: &mut [u8], input: &LSF) {
    let mut ud = [0u8; 240 + 4 + 4];

    for i in 0..8 {
        ud[4 + i] = (input.dst[0] >> (7 - i)) & 1;
        ud[4 + i + 8] = (input.dst[1] >> (7 - i)) & 1;
        ud[4 + i + 16] = (input.dst[2] >> (7 - i)) & 1;
        ud[4 + i + 24] = (input.dst[3] >> (7 - i)) & 1;
        ud[4 + i + 32] = (input.dst[4] >> (7 - i)) & 1;
        ud[4 + i + 40] = (input.dst[5] >> (7 - i)) & 1;
    }

    for i in 0..8 {
        ud[4 + i + 48] = (input.src[0] >> (7 - i)) & 1;
        ud[4 + i + 56] = (input.src[1] >> (7 - i)) & 1;
        ud[4 + i + 64] = (input.src[2] >> (7 - i)) & 1;
        ud[4 + i + 72] = (input.src[3] >> (7 - i)) & 1;
        ud[4 + i + 80] = (input.src[4] >> (7 - i)) & 1;
        ud[4 + i + 88] = (input.src[5] >> (7 - i)) & 1;
    }

    for i in 0..8 {
        ud[4 + i + 96] = (input.type_field[0] >> (7 - i)) & 1;
        ud[4 + i + 104] = (input.type_field[1] >> (7 - i)) & 1;
    }

    for i in 0..8 {
        ud[4 + i + 112] = (input.meta[0] >> (7 - i)) & 1;
        ud[4 + i + 120] = (input.meta[1] >> (7 - i)) & 1;
        ud[4 + i + 128] = (input.meta[2] >> (7 - i)) & 1;
        ud[4 + i + 136] = (input.meta[3] >> (7 - i)) & 1;
        ud[4 + i + 144] = (input.meta[4] >> (7 - i)) & 1;
        ud[4 + i + 152] = (input.meta[5] >> (7 - i)) & 1;
        ud[4 + i + 160] = (input.meta[6] >> (7 - i)) & 1;
        ud[4 + i + 168] = (input.meta[7] >> (7 - i)) & 1;
        ud[4 + i + 176] = (input.meta[8] >> (7 - i)) & 1;
        ud[4 + i + 184] = (input.meta[9] >> (7 - i)) & 1;
        ud[4 + i + 192] = (input.meta[10] >> (7 - i)) & 1;
        ud[4 + i + 200] = (input.meta[11] >> (7 - i)) & 1;
        ud[4 + i + 208] = (input.meta[12] >> (7 - i)) & 1;
        ud[4 + i + 216] = (input.meta[13] >> (7 - i)) & 1;
    }

    for i in 0..8 {
        ud[4 + i + 224] = (input.crc[0] >> (7 - i)) & 1;
        ud[4 + i + 232] = (input.crc[1] >> (7 - i)) & 1;
    }

    encode_convolutional(out, &ud, 240 + 4, &PUNCTURE_PATTERN_1);
}

fn encode_convolutional(out: &mut [u8], ud: &[u8], useful_len: usize, puncture_pattern: &[u8]) {
    let mut p = 0usize;
    let mut pb = 0usize;

    for i in 0..useful_len {
        let g1 = (ud[i + 4] + ud[i + 1] + ud[i]) % 2;
        let g2 = (ud[i + 4] + ud[i + 3] + ud[i + 2] + ud[i]) % 2;

        if puncture_pattern[p] != 0 {
            out[pb] = g1;
            pb += 1;
        }
        p = (p + 1) % puncture_pattern.len();

        if puncture_pattern[p] != 0 {
            out[pb] = g2;
            pb += 1;
        }
        p = (p + 1) % puncture_pattern.len();
    }
}
