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
        let idx = (input[2 * i] as usize) * 2 + input[2 * i + 1] as usize;
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
        // PREAM_LSF
        for _ in 0..(SYM_PER_FRA / 2) {
            out[*cnt as usize] = 3.0;
            *cnt += 1;
            out[*cnt as usize] = -3.0;
            *cnt += 1;
        }
    }
}

pub fn send_syncword(out: &mut [f32; SYM_PER_SWD], cnt: &mut u32, syncword: u16) {
    let mut i: u8 = 0;
    while i < (SYM_PER_SWD * 2) as u8 {
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
    let mut enc_bits = [0u8; SYM_PER_PLD * 2];
    let mut rf_bits = [0u8; SYM_PER_PLD * 2];
    let mut sym_cnt: u32 = 0;

    // The send_syncword function expects [f32; SYM_PER_SWD]; we wrap the start of out.
    // But out is [f32; SYM_PER_FRA]. We need to write 8 syncword symbols at the
    // beginning, then 184 payload symbols. Use a helper.

    let sync_value = match frame_type {
        FrameType::Lsf => {
            let mut tmp_lich = [0u8; 6];
            // For LSF, use SYNC_LSF and conv_encode_lsf
            conv_encode_lsf(&mut enc_bits, lsf);
            let _ = tmp_lich;
            SYNC_LSF
        }
        FrameType::Str => {
            let mut lich = [0u8; 6];
            extract_lich(&mut lich, lich_cnt, lsf);
            let lich_encoded = encode_LICH(&lich);
            unpack_lich(&mut enc_bits, &lich_encoded);
            // stream frames require 16-byte payloads
            // The stream encoding writes to enc_bits[96..]
            conv_encode_stream_frame(&mut enc_bits[96..], data, fn_num);
            SYNC_STR
        }
        FrameType::Pkt => {
            conv_encode_packet_frame(&mut enc_bits, data);
            SYNC_PKT
        }
    };

    // send syncword to first 8 of out
    let mut i: u8 = 0;
    while i < (SYM_PER_SWD * 2) as u8 {
        let idx = ((sync_value >> (14 - i)) & 3) as usize;
        out[sym_cnt as usize] = SYMBOL_MAP[idx] as f32;
        sym_cnt += 1;
        i += 2;
    }

    // common stuff
    reorder_bits(&mut rf_bits, &enc_bits);
    randomize_bits(&mut rf_bits);

    // send_data writes SYM_PER_PLD symbols
    for i in 0..SYM_PER_PLD {
        let idx = (rf_bits[2 * i] as usize) * 2 + rf_bits[2 * i + 1] as usize;
        out[sym_cnt as usize] = SYMBOL_MAP[idx] as f32;
        sym_cnt += 1;
    }
}
