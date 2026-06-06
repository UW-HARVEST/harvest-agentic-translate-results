pub mod decode;
pub mod encode;
pub mod math;
pub mod payload;
pub mod phy;
pub mod types;

use crate::encode::{conv_encode_lsf, conv_encode_packet_frame, conv_encode_stream_frame, EOT_SYMBOLS, SYMBOL_MAP};
use crate::payload::{extract_lich, unpack_lich};
use crate::phy::{randomize_bits, reorder_bits, SYNC_LSF, SYNC_PKT, SYNC_STR};
use crate::math::encode_LICH;
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
    let mut i: u32 = 0;
    while i < (SYM_PER_SWD as u32) * 2 {
        let dibit = (syncword >> (14 - i)) & 3;
        out[*cnt as usize] = SYMBOL_MAP[dibit as usize] as f32;
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
    let mut enc_bits: [u8; SYM_PER_PLD * 2] = [0; SYM_PER_PLD * 2];
    let mut rf_bits: [u8; SYM_PER_PLD * 2] = [0; SYM_PER_PLD * 2];
    let mut sym_cnt: u32 = 0;

    // Pull off the first 8 symbols (syncword) and the 184 payload symbols
    // We send the syncword into the first 8 entries of `out`, then the payload
    // bits become the remaining 184 symbols. The C version uses the same
    // pointer/buffer for the whole frame.

    match frame_type {
        FrameType::Lsf => {
            // Write the syncword into out[0..8].
            send_syncword_into(out, &mut sym_cnt, SYNC_LSF);
            conv_encode_lsf(&mut enc_bits, lsf);
        }
        FrameType::Str => {
            send_syncword_into(out, &mut sym_cnt, SYNC_STR);
            let mut lich: [u8; 6] = [0; 6];
            extract_lich(&mut lich, lich_cnt, lsf);
            let lich_encoded = encode_LICH(&lich);
            // unpack the encoded LICH into the first 96 entries of enc_bits
            unpack_lich(&mut enc_bits[..96], &lich_encoded);
            // encode the payload (16 bytes) into the rest
            conv_encode_stream_frame(&mut enc_bits[96..], data, fn_num);
        }
        FrameType::Pkt => {
            send_syncword_into(out, &mut sym_cnt, SYNC_PKT);
            conv_encode_packet_frame(&mut enc_bits, data);
        }
    }

    // common stuff
    reorder_bits(&mut rf_bits, &enc_bits);
    randomize_bits(&mut rf_bits);

    // Write the payload symbols into out[sym_cnt..]
    for i in 0..SYM_PER_PLD {
        let idx = (rf_bits[2 * i] * 2 + rf_bits[2 * i + 1]) as usize;
        out[sym_cnt as usize] = SYMBOL_MAP[idx] as f32;
        sym_cnt += 1;
    }
}

// Helper: write a syncword into a generic-length slice buffer (used by send_frame).
fn send_syncword_into(out: &mut [f32], cnt: &mut u32, syncword: u16) {
    let mut i: u32 = 0;
    while i < (SYM_PER_SWD as u32) * 2 {
        let dibit = (syncword >> (14 - i)) & 3;
        out[*cnt as usize] = SYMBOL_MAP[dibit as usize] as f32;
        *cnt += 1;
        i += 2;
    }
}
