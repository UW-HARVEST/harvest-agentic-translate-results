use crate::math::q_abs_diff;

// Synchronization symbols for LSF (Link Setup Frame)
pub const LSF_SYNC_SYMBOLS: [i8; 8] = [3, 3, 3, 3, -3, -3, 3, -3];
// Synchronization symbols for Stream
pub const STR_SYNC_SYMBOLS: [i8; 8] = [-3, -3, -3, -3, 3, 3, -3, 3];
// Synchronization symbols for Packet
pub const PKT_SYNC_SYMBOLS: [i8; 8] = [3, -3, 3, 3, -3, -3, -3, -3];
// Symbol levels for modulation
pub const SYMBOL_LEVELS: [f32; 4] = [-3.0, -1.0, 1.0, 3.0];
pub const NUM_STATES: usize = 1 << (5 - 1);
static mut PREV_METRICS: [u32; NUM_STATES] = [0; NUM_STATES];
static mut CURR_METRICS: [u32; NUM_STATES] = [0; NUM_STATES];
static mut PREV_METRICS_DATA: [u32; NUM_STATES] = [0; NUM_STATES];
static mut CURR_METRICS_DATA: [u32; NUM_STATES] = [0; NUM_STATES];
static mut VITERBI_HISTORY: [u16; 244] = [0; 244];

pub fn viterbi_decode(out: &mut [u8], input: &[u16], len: u16) -> u32 {
    viterbi_reset();
    let mut pos: usize = 0;
    let mut i: usize = 0;
    while i < len as usize {
        let s0 = input[i];
        let s1 = input[i + 1];
        viterbi_decode_bit(s0, s1, pos);
        pos += 1;
        i += 2;
    }
    viterbi_chainback(out, pos, len / 2)
}

pub fn viterbi_decode_punctured(
    out: &mut [u8],
    input: &[u16],
    punct: &[u8],
    in_len: u16,
    p_len: u16,
) -> u32 {
    let mut umsg = [0u16; 244 * 2];
    let mut p: usize = 0;
    let mut u: usize = 0;
    let mut i: usize = 0;

    while i < in_len as usize {
        if punct[p] != 0 {
            umsg[u] = input[i];
            i += 1;
        } else {
            umsg[u] = 0x7FFF;
        }
        u += 1;
        p += 1;
        p %= p_len as usize;
    }

    let cost = viterbi_decode(out, &umsg, u as u16);
    cost - ((u as u32) - (in_len as u32)) * 0x7FFF
}

pub fn viterbi_decode_bit(s0: u16, s1: u16, pos: usize) {
    const COST_TABLE_0: [u16; 8] = [0, 0, 0, 0, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF];
    const COST_TABLE_1: [u16; 8] = [0, 0xFFFF, 0xFFFF, 0, 0, 0xFFFF, 0xFFFF, 0];

    unsafe {
        for i in 0..(NUM_STATES / 2) {
            let metric = q_abs_diff(COST_TABLE_0[i], s0) as u32
                + q_abs_diff(COST_TABLE_1[i], s1) as u32;

            let m0 = PREV_METRICS[i] + metric;
            let m1 = PREV_METRICS[i + NUM_STATES / 2] + (0x1FFFE - metric);
            let m2 = PREV_METRICS[i] + (0x1FFFE - metric);
            let m3 = PREV_METRICS[i + NUM_STATES / 2] + metric;

            let i0 = 2 * i;
            let i1 = i0 + 1;

            if m0 >= m1 {
                VITERBI_HISTORY[pos] |= 1 << i0;
                CURR_METRICS[i0] = m1;
            } else {
                VITERBI_HISTORY[pos] &= !(1 << i0);
                CURR_METRICS[i0] = m0;
            }

            if m2 >= m3 {
                VITERBI_HISTORY[pos] |= 1 << i1;
                CURR_METRICS[i1] = m3;
            } else {
                VITERBI_HISTORY[pos] &= !(1 << i1);
                CURR_METRICS[i1] = m2;
            }
        }

        // swap
        let mut tmp = [0u32; NUM_STATES];
        for i in 0..NUM_STATES {
            tmp[i] = CURR_METRICS[i];
        }
        for i in 0..NUM_STATES {
            CURR_METRICS[i] = PREV_METRICS[i];
            PREV_METRICS[i] = tmp[i];
        }
    }
}

fn viterbi_chainback(out: &mut [u8], mut pos: usize, len: u16) -> u32 {
    let mut state: u8 = 0;
    let mut bit_pos = len as usize + 4;

    // zero out output
    let out_len = ((len as usize).wrapping_sub(1)) / 8 + 1;
    for b in out[..out_len].iter_mut() {
        *b = 0;
    }

    unsafe {
        while pos > 0 {
            bit_pos -= 1;
            pos -= 1;
            let bit = VITERBI_HISTORY[pos] & (1 << (state >> 4));
            state >>= 1;
            if bit != 0 {
                state |= 0x80;
                out[bit_pos / 8] |= 1 << (7 - (bit_pos % 8));
            }
        }

        let mut cost = PREV_METRICS[0];
        for i in 0..NUM_STATES {
            if PREV_METRICS[i] < cost {
                cost = PREV_METRICS[i];
            }
        }
        cost
    }
}

fn viterbi_reset() {
    unsafe {
        VITERBI_HISTORY = [0u16; 244];
        CURR_METRICS = [0u32; NUM_STATES];
        PREV_METRICS = [0u32; NUM_STATES];
        CURR_METRICS_DATA = [0u32; NUM_STATES];
        PREV_METRICS_DATA = [0u32; NUM_STATES];
    }
}
