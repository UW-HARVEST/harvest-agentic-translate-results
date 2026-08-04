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
    let pp_len = PUNCTURE_PATTERN_2.len();
    let mut p: usize = 0;
    let mut pb: usize = 0;
    let mut ud = [0u8; 144 + 4 + 4];

    // unpack frame number (16 bits, MSB first)
    for i in 0..16 {
        ud[4 + i] = ((fn_num >> (15 - i)) & 1) as u8;
    }

    // unpack data (16 bytes)
    for i in 0..16 {
        for j in 0..8 {
            ud[4 + 16 + i * 8 + j] = (input[i] >> (7 - j)) & 1;
        }
    }

    // encode
    for i in 0..(144 + 4) {
        let g1 = (ud[i + 4] + ud[i + 1] + ud[i + 0]) % 2;
        let g2 = (ud[i + 4] + ud[i + 3] + ud[i + 2] + ud[i + 0]) % 2;

        if PUNCTURE_PATTERN_2[p] != 0 {
            out[pb] = g1;
            pb += 1;
        }
        p = (p + 1) % pp_len;

        if PUNCTURE_PATTERN_2[p] != 0 {
            out[pb] = g2;
            pb += 1;
        }
        p = (p + 1) % pp_len;
    }
}
pub fn conv_encode_packet_frame(out: &mut [u8], input: &[u8]) {
    let pp_len = PUNCTURE_PATTERN_3.len();
    let mut p: usize = 0;
    let mut pb: usize = 0;
    let mut ud = [0u8; 206 + 4 + 4];

    // unpack data (26 bytes, last byte only 6 bits)
    for i in 0..26usize {
        for j in 0..8usize {
            if i <= 24 || j <= 5 {
                ud[4 + i * 8 + j] = (input[i] >> (7 - j)) & 1;
            }
        }
    }

    // encode
    for i in 0..(206 + 4) {
        let g1 = (ud[i + 4] + ud[i + 1] + ud[i + 0]) % 2;
        let g2 = (ud[i + 4] + ud[i + 3] + ud[i + 2] + ud[i + 0]) % 2;

        if PUNCTURE_PATTERN_3[p] != 0 {
            out[pb] = g1;
            pb += 1;
        }
        p = (p + 1) % pp_len;

        if PUNCTURE_PATTERN_3[p] != 0 {
            out[pb] = g2;
            pb += 1;
        }
        p = (p + 1) % pp_len;
    }
}
pub fn conv_encode_lsf(out: &mut [u8], input: &LSF) {
    let pp_len = PUNCTURE_PATTERN_1.len();
    let mut p: usize = 0;
    let mut pb: usize = 0;
    let mut ud = [0u8; 240 + 4 + 4];

    // unpack DST (6 bytes)
    for i in 0..8 {
        ud[4 + i] = (input.dst[0] >> (7 - i)) & 1;
        ud[4 + i + 8] = (input.dst[1] >> (7 - i)) & 1;
        ud[4 + i + 16] = (input.dst[2] >> (7 - i)) & 1;
        ud[4 + i + 24] = (input.dst[3] >> (7 - i)) & 1;
        ud[4 + i + 32] = (input.dst[4] >> (7 - i)) & 1;
        ud[4 + i + 40] = (input.dst[5] >> (7 - i)) & 1;
    }

    // unpack SRC (6 bytes)
    for i in 0..8 {
        ud[4 + i + 48] = (input.src[0] >> (7 - i)) & 1;
        ud[4 + i + 56] = (input.src[1] >> (7 - i)) & 1;
        ud[4 + i + 64] = (input.src[2] >> (7 - i)) & 1;
        ud[4 + i + 72] = (input.src[3] >> (7 - i)) & 1;
        ud[4 + i + 80] = (input.src[4] >> (7 - i)) & 1;
        ud[4 + i + 88] = (input.src[5] >> (7 - i)) & 1;
    }

    // unpack TYPE (2 bytes)
    for i in 0..8 {
        ud[4 + i + 96] = (input.type_field[0] >> (7 - i)) & 1;
        ud[4 + i + 104] = (input.type_field[1] >> (7 - i)) & 1;
    }

    // unpack META (14 bytes)
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

    // unpack CRC (2 bytes)
    for i in 0..8 {
        ud[4 + i + 224] = (input.crc[0] >> (7 - i)) & 1;
        ud[4 + i + 232] = (input.crc[1] >> (7 - i)) & 1;
    }

    // encode
    for i in 0..(240 + 4) {
        let g1 = (ud[i + 4] + ud[i + 1] + ud[i + 0]) % 2;
        let g2 = (ud[i + 4] + ud[i + 3] + ud[i + 2] + ud[i + 0]) % 2;

        if PUNCTURE_PATTERN_1[p] != 0 {
            out[pb] = g1;
            pb += 1;
        }
        p = (p + 1) % pp_len;

        if PUNCTURE_PATTERN_1[p] != 0 {
            out[pb] = g2;
            pb += 1;
        }
        p = (p + 1) % pp_len;
    }
}
