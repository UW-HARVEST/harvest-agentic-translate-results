pub mod decode;
pub mod encode;
pub mod math;
pub mod payload;
pub mod phy;
pub mod types;

use crate::types::{FrameType, PreambleType, LSF, SYM_PER_FRA, SYM_PER_PLD, SYM_PER_SWD};

pub fn send_data(out: &mut [f32; SYM_PER_PLD], cnt: &mut u32, input: &[u8]) {
    for i in 0..SYM_PER_PLD {
        out[*cnt as usize] = encode::SYMBOL_MAP[(input[2 * i] * 2 + input[2 * i + 1]) as usize] as f32;
        *cnt += 1;
    }
}

pub fn send_eot(out: &mut [f32; SYM_PER_FRA], cnt: &mut u32) {
    for i in 0..SYM_PER_FRA {
        out[*cnt as usize] = encode::EOT_SYMBOLS[i % 8];
        *cnt += 1;
    }
}

pub fn send_preamble(out: &mut [f32; SYM_PER_FRA], cnt: &mut u32, ptype: PreambleType) {
    for _ in 0..(SYM_PER_FRA / 2) {
        match ptype {
            PreambleType::Bert => {
                out[*cnt as usize] = -3.0;
                *cnt += 1;
                out[*cnt as usize] = 3.0;
                *cnt += 1;
            }
            PreambleType::Lsf => {
                out[*cnt as usize] = 3.0;
                *cnt += 1;
                out[*cnt as usize] = -3.0;
                *cnt += 1;
            }
        }
    }
}

pub fn send_syncword(out: &mut [f32; SYM_PER_SWD], cnt: &mut u32, syncword: u16) {
    for i in (0..(SYM_PER_SWD * 2)).step_by(2) {
        out[*cnt as usize] = encode::SYMBOL_MAP[((syncword >> (14 - i)) & 3) as usize] as f32;
        *cnt += 1;
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

    match frame_type {
        FrameType::Lsf => {
            let mut sync = [0.0f32; SYM_PER_SWD];
            let mut sync_cnt = 0u32;
            send_syncword(&mut sync, &mut sync_cnt, phy::SYNC_LSF);
            out[..SYM_PER_SWD].copy_from_slice(&sync);
            encode::conv_encode_lsf(&mut enc_bits, lsf);
        }
        FrameType::Str => {
            let mut sync = [0.0f32; SYM_PER_SWD];
            let mut sync_cnt = 0u32;
            send_syncword(&mut sync, &mut sync_cnt, phy::SYNC_STR);
            out[..SYM_PER_SWD].copy_from_slice(&sync);
            payload::extract_lich(&mut lich, lich_cnt, lsf);
            let lich_encoded = math::encode_LICH(&lich);
            payload::unpack_lich(&mut enc_bits, &lich_encoded);
            encode::conv_encode_stream_frame(&mut enc_bits[96..], data, fn_num);
        }
        FrameType::Pkt => {
            let mut sync = [0.0f32; SYM_PER_SWD];
            let mut sync_cnt = 0u32;
            send_syncword(&mut sync, &mut sync_cnt, phy::SYNC_PKT);
            out[..SYM_PER_SWD].copy_from_slice(&sync);
            encode::conv_encode_packet_frame(&mut enc_bits, data);
        }
    }

    phy::reorder_bits(&mut rf_bits, &enc_bits);
    phy::randomize_bits(&mut rf_bits);
    let mut payload = [0.0f32; SYM_PER_PLD];
    let mut payload_cnt = 0u32;
    send_data(&mut payload, &mut payload_cnt, &rf_bits);
    out[SYM_PER_SWD..].copy_from_slice(&payload);
}
