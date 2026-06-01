pub mod decode;
pub mod encode;
pub mod math;
pub mod payload;
pub mod phy;
pub mod types;

use crate::types::{FrameType, PreambleType, LSF, SYM_PER_FRA, SYM_PER_PLD, SYM_PER_SWD};

pub fn send_data(out: &mut [f32; SYM_PER_PLD], cnt: &mut u32, input: &[u8]) {
    for i in 0..SYM_PER_PLD {
        let idx = (input[2 * i] * 2 + input[2 * i + 1]) as usize;
        out[*cnt as usize] = encode::SYMBOL_MAP[idx] as f32;
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
    let mut i: u8 = 0;
    while i < (SYM_PER_SWD as u8) * 2 {
        let idx = ((syncword >> (14 - i)) & 3) as usize;
        out[*cnt as usize] = encode::SYMBOL_MAP[idx] as f32;
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

    // Build a syncword buffer that we later copy into out[0..8]
    let mut sync_buf: [f32; SYM_PER_SWD] = [0.0; SYM_PER_SWD];
    let mut sync_cnt: u32 = 0;

    match frame_type {
        FrameType::Lsf => {
            send_syncword(&mut sync_buf, &mut sync_cnt, phy::SYNC_LSF);
            encode::conv_encode_lsf(&mut enc_bits, lsf);
        }
        FrameType::Str => {
            send_syncword(&mut sync_buf, &mut sync_cnt, phy::SYNC_STR);
            let mut lich: [u8; 6] = [0; 6];
            payload::extract_lich(&mut lich, lich_cnt, lsf);
            let lich_encoded = crate::math::encode_LICH(&lich);
            payload::unpack_lich(&mut enc_bits, &lich_encoded);
            // The remaining bits start at index 96
            let mut tail = [0u8; SYM_PER_PLD * 2 - 96];
            encode::conv_encode_stream_frame(&mut tail, data, fn_num);
            enc_bits[96..(96 + tail.len())].copy_from_slice(&tail);
        }
        FrameType::Pkt => {
            send_syncword(&mut sync_buf, &mut sync_cnt, phy::SYNC_PKT);
            encode::conv_encode_packet_frame(&mut enc_bits, data);
        }
    }

    // copy syncword to output
    for i in 0..SYM_PER_SWD {
        out[sym_cnt as usize] = sync_buf[i];
        sym_cnt += 1;
    }

    phy::reorder_bits(&mut rf_bits, &enc_bits);
    phy::randomize_bits(&mut rf_bits);

    // We need to send_data into a buffer of SYM_PER_PLD floats. The output is
    // SYM_PER_FRA = 192 floats and 8 are already used by syncword, leaving 184 = SYM_PER_PLD.
    let mut pld_buf: [f32; SYM_PER_PLD] = [0.0; SYM_PER_PLD];
    let mut pld_cnt: u32 = 0;
    send_data(&mut pld_buf, &mut pld_cnt, &rf_bits);
    for i in 0..SYM_PER_PLD {
        out[sym_cnt as usize] = pld_buf[i];
        sym_cnt += 1;
    }
}
