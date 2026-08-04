pub mod decode;
pub mod encode;
pub mod math;
pub mod payload;
pub mod phy;
pub mod types;

use crate::encode::{
    conv_encode_lsf, conv_encode_packet_frame, conv_encode_stream_frame, EOT_SYMBOLS, SYMBOL_MAP,
};
use crate::payload::{extract_lich, unpack_lich};
use crate::math::encode_LICH;
use crate::phy::{randomize_bits, reorder_bits, SYNC_LSF, SYNC_PKT, SYNC_STR};
use crate::types::{FrameType, PreambleType, LSF, SYM_PER_FRA, SYM_PER_PLD, SYM_PER_SWD};

pub fn send_data(out: &mut [f32; SYM_PER_PLD], cnt: &mut u32, input: &[u8]) {
    for i in 0..SYM_PER_PLD {
        let idx = (input[2 * i] * 2 + input[2 * i + 1]) as usize;
        out[*cnt as usize] = SYMBOL_MAP[idx] as f32;
        *cnt += 1;
    }
}

pub fn send_eot(out: &mut [f32; SYM_PER_FRA], cnt: &mut u32) {
    for i in 0..SYM_PER_FRA {
        out[*cnt as usize] = EOT_SYMBOLS[i % 8];
        *cnt += 1;
    }
}

pub fn send_preamble(out: &mut [f32; SYM_PER_FRA], cnt: &mut u32, ptype: PreambleType) {
    if ptype == PreambleType::Bert {
        for _ in 0..(SYM_PER_FRA / 2) {
            out[*cnt as usize] = -3.0;
            *cnt += 1;
            out[*cnt as usize] = 3.0;
            *cnt += 1;
        }
    } else {
        for _ in 0..(SYM_PER_FRA / 2) {
            out[*cnt as usize] = 3.0;
            *cnt += 1;
            out[*cnt as usize] = -3.0;
            *cnt += 1;
        }
    }
}

pub fn send_syncword(out: &mut [f32; SYM_PER_SWD], cnt: &mut u32, syncword: u16) {
    let mut i = 0u8;
    while i < (SYM_PER_SWD as u8) * 2 {
        let idx = ((syncword >> (14 - i)) & 3) as usize;
        out[*cnt as usize] = SYMBOL_MAP[idx] as f32;
        *cnt += 1;
        i += 2;
    }
}

pub fn send_frame(
    out: &mut [f32; SYM_PER_FRA],
    data: &[u8],
    frame_type: FrameType,
    lsf: &LSF,
    lich_cnt: u8,
    fn_num: u16,
) {
    let mut lich = [0u8; 6];
    let mut enc_bits = [0u8; SYM_PER_PLD * 2];
    let mut rf_bits = [0u8; SYM_PER_PLD * 2];
    let mut sym_cnt: u32 = 0;

    // Use a separate small buffer for syncword, then copy into out
    let mut sw_buf = [0f32; SYM_PER_SWD];
    let mut sw_cnt: u32 = 0;

    match frame_type {
        FrameType::Lsf => {
            send_syncword(&mut sw_buf, &mut sw_cnt, SYNC_LSF);
            conv_encode_lsf(&mut enc_bits, lsf);
        }
        FrameType::Str => {
            send_syncword(&mut sw_buf, &mut sw_cnt, SYNC_STR);
            extract_lich(&mut lich, lich_cnt, lsf);
            let lich_encoded = encode_LICH(&lich);
            unpack_lich(&mut enc_bits, &lich_encoded);
            // stream frames require 16-byte payloads
            conv_encode_stream_frame(&mut enc_bits[96..], data, fn_num);
        }
        FrameType::Pkt => {
            send_syncword(&mut sw_buf, &mut sw_cnt, SYNC_PKT);
            conv_encode_packet_frame(&mut enc_bits, data);
        }
    }

    // copy syncword symbols into out
    for i in 0..SYM_PER_SWD {
        out[sym_cnt as usize] = sw_buf[i];
        sym_cnt += 1;
    }

    // common stuff
    reorder_bits(&mut rf_bits, &enc_bits);
    randomize_bits(&mut rf_bits);

    // Replicate send_data using out (SYM_PER_FRA) starting at sym_cnt.
    for i in 0..SYM_PER_PLD {
        let idx = (rf_bits[2 * i] * 2 + rf_bits[2 * i + 1]) as usize;
        out[sym_cnt as usize] = SYMBOL_MAP[idx] as f32;
        sym_cnt += 1;
    }
}
