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
    if ptype == PreambleType::Bert {
        for _ in 0..SYM_PER_FRA / 2 {
            out[*cnt as usize] = -3.0;
            *cnt += 1;
            out[*cnt as usize] = 3.0;
            *cnt += 1;
        }
    } else {
        for _ in 0..SYM_PER_FRA / 2 {
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
        out[*cnt as usize] = encode::SYMBOL_MAP[((syncword >> (14 - i)) & 3) as usize] as f32;
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

    // We need to cast the SYM_PER_FRA output buffer to SYM_PER_SWD for send_syncword
    // Since send_syncword writes to out[0..8] via cnt, we use a temporary buffer
    let mut sw_buf = [0f32; SYM_PER_SWD];

    match frame_type {
        FrameType::Lsf => {
            send_syncword(&mut sw_buf, &mut sym_cnt, phy::SYNC_LSF);
            encode::conv_encode_lsf(&mut enc_bits, lsf);
        }
        FrameType::Str => {
            send_syncword(&mut sw_buf, &mut sym_cnt, phy::SYNC_STR);
            let mut lich = [0u8; 6];
            payload::extract_lich(&mut lich, lich_cnt, lsf);
            let lich_encoded = math::encode_LICH(&lich);
            payload::unpack_lich(&mut enc_bits[..96], &lich_encoded);
            encode::conv_encode_stream_frame(&mut enc_bits[96..], data, fn_num);
        }
        FrameType::Pkt => {
            send_syncword(&mut sw_buf, &mut sym_cnt, phy::SYNC_PKT);
            encode::conv_encode_packet_frame(&mut enc_bits, data);
        }
    }

    // Copy syncword symbols to output
    for i in 0..SYM_PER_SWD {
        out[i] = sw_buf[i];
    }

    // common stuff
    phy::reorder_bits(&mut rf_bits, &enc_bits);
    phy::randomize_bits(&mut rf_bits);

    // send_data expects &mut [f32; SYM_PER_PLD], we need to write starting at sym_cnt
    let mut data_buf = [0f32; SYM_PER_PLD];
    let mut data_cnt: u32 = 0;
    send_data(&mut data_buf, &mut data_cnt, &rf_bits);

    for i in 0..SYM_PER_PLD {
        out[sym_cnt as usize + i] = data_buf[i];
    }
}
