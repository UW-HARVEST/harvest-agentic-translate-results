use crate::aes::{NB, NR, NK, SBOX, RCON};

pub fn add(state: &mut [[u8; NB]; 4], round_key: &[[u8; NB]; 4]) {
    for i in 0..4 {
        for j in 0..NB {
            state[i][j] ^= round_key[i][j];
        }
    }
}

pub fn expansion(key: &[u8; 4 * NK], w: &mut [u8; 4 * NB * (NR + 1)]) {
    // Zero out w
    for byte in w.iter_mut() {
        *byte = 0;
    }

    // Initial loop: replicate the C SSE behavior with overlapping stores.
    // For each i in 0..Nk*4 step 16, store the 16-byte key at w[i], w[i+4], w[i+8], w[i+12]
    let mut i = 0usize;
    while i < NK * 4 {
        let temp: [u8; 16] = {
            let mut buf = [0u8; 16];
            buf.copy_from_slice(&key[i..i + 16]);
            buf
        };
        // Store at w[i], w[i+4], w[i+8], w[i+12], with later stores overwriting earlier ones
        for offset in [0usize, 4, 8, 12].iter() {
            for k in 0..16 {
                w[i + offset + k] = temp[k];
            }
        }
        i += 16;
    }

    let mut i = NK;
    while i < NB * (NR + 1) {
        // Load 16 bytes starting at w[4 * (i - 1)]
        let load_pos = 4 * (i - 1);
        let mut temp_bytes = [0u8; 16];
        for k in 0..16 {
            temp_bytes[k] = w[load_pos + k];
        }

        if i % NK == 0 {
            // _mm_shuffle_epi32(temp, _MM_SHUFFLE(3, 0, 1, 2))
            // Treats temp as four 32-bit lanes, reorders them.
            // _MM_SHUFFLE(d,c,b,a): result_lane_3 = src_lane_d (=3),
            //                       result_lane_2 = src_lane_c (=0),
            //                       result_lane_1 = src_lane_b (=1),
            //                       result_lane_0 = src_lane_a (=2)
            let lane: [[u8; 4]; 4] = [
                [temp_bytes[0], temp_bytes[1], temp_bytes[2], temp_bytes[3]],
                [temp_bytes[4], temp_bytes[5], temp_bytes[6], temp_bytes[7]],
                [temp_bytes[8], temp_bytes[9], temp_bytes[10], temp_bytes[11]],
                [temp_bytes[12], temp_bytes[13], temp_bytes[14], temp_bytes[15]],
            ];
            // new lane order: [lane2, lane1, lane0, lane3]
            let new_lanes = [lane[2], lane[1], lane[0], lane[3]];
            for li in 0..4 {
                for bi in 0..4 {
                    temp_bytes[li * 4 + bi] = new_lanes[li][bi];
                }
            }

            for j in 0..4 {
                temp_bytes[j] = SBOX[temp_bytes[j] as usize];
            }
            temp_bytes[0] ^= RCON[i / NK];
        } else if NK > 6 && (i % NK == 4) {
            for j in 0..4 {
                temp_bytes[j] = SBOX[temp_bytes[j] as usize];
                temp_bytes[j + 1] = SBOX[temp_bytes[j + 1] as usize];
                temp_bytes[j + 2] = SBOX[temp_bytes[j + 2] as usize];
                temp_bytes[j + 3] = SBOX[temp_bytes[j + 3] as usize];
            }
        }

        // XOR with w[4 * (i - Nk)..4 * (i - Nk) + 16]
        let xor_pos = 4 * (i - NK);
        let mut xor_result = [0u8; 16];
        for k in 0..16 {
            xor_result[k] = temp_bytes[k] ^ w[xor_pos + k];
        }

        // Store at w[4 * i..4 * i + 16]
        let store_pos = 4 * i;
        for k in 0..16 {
            w[store_pos + k] = xor_result[k];
        }

        i += 4;
    }
}
