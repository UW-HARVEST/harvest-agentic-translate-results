use std::sync::{Mutex, OnceLock};

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

#[derive(Clone)]
struct ViterbiState {
    prev_metrics: [u32; NUM_STATES],
    curr_metrics: [u32; NUM_STATES],
    prev_metrics_data: [u32; NUM_STATES],
    curr_metrics_data: [u32; NUM_STATES],
    viterbi_history: [u16; 244],
}

impl Default for ViterbiState {
    fn default() -> Self {
        Self {
            prev_metrics: [0; NUM_STATES],
            curr_metrics: [0; NUM_STATES],
            prev_metrics_data: [0; NUM_STATES],
            curr_metrics_data: [0; NUM_STATES],
            viterbi_history: [0; 244],
        }
    }
}

fn state() -> &'static Mutex<ViterbiState> {
    static STATE: OnceLock<Mutex<ViterbiState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(ViterbiState::default()))
}

pub fn viterbi_decode(out: &mut [u8], input: &[u16], len: u16) -> u32 {
    viterbi_reset();

    let mut pos = 0usize;
    for i in (0..len as usize).step_by(2) {
        viterbi_decode_bit(input[i], input[i + 1], pos);
        pos += 1;
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
    let mut p = 0usize;
    let mut u = 0usize;
    let mut i = 0usize;

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

    viterbi_decode(out, &umsg, u as u16).wrapping_sub(((u - in_len as usize) as u32) * 0x7FFF)
}
pub fn viterbi_decode_bit(s0: u16, s1: u16, pos: usize) {
    const COST_TABLE_0: [u16; 8] = [0, 0, 0, 0, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF];
    const COST_TABLE_1: [u16; 8] = [0, 0xFFFF, 0xFFFF, 0, 0, 0xFFFF, 0xFFFF, 0];

    let mut state = state().lock().expect("viterbi mutex poisoned");

    for i in 0..(NUM_STATES / 2) {
        let metric = q_abs_diff(COST_TABLE_0[i], s0) as u32 + q_abs_diff(COST_TABLE_1[i], s1) as u32;

        let m0 = state.prev_metrics[i] + metric;
        let m1 = state.prev_metrics[i + NUM_STATES / 2] + (0x1FFFE - metric);
        let m2 = state.prev_metrics[i] + (0x1FFFE - metric);
        let m3 = state.prev_metrics[i + NUM_STATES / 2] + metric;

        let i0 = 2 * i;
        let i1 = i0 + 1;

        if m0 >= m1 {
            state.viterbi_history[pos] |= 1 << i0;
            state.curr_metrics[i0] = m1;
        } else {
            state.viterbi_history[pos] &= !(1 << i0);
            state.curr_metrics[i0] = m0;
        }

        if m2 >= m3 {
            state.viterbi_history[pos] |= 1 << i1;
            state.curr_metrics[i1] = m3;
        } else {
            state.viterbi_history[pos] &= !(1 << i1);
            state.curr_metrics[i1] = m2;
        }
    }

    let tmp = state.curr_metrics;
    state.curr_metrics = state.prev_metrics;
    state.prev_metrics = tmp;
}
fn viterbi_chainback(out: &mut [u8], pos: usize, len: u16) -> u32 {
    let state = state().lock().expect("viterbi mutex poisoned");
    let mut state_idx = 0u8;
    let mut bit_pos = len as usize + 4;
    let bytes = ((len as usize).saturating_sub(1) / 8) + 1;
    let zero_len = out.len().min(bytes);

    out[..zero_len].fill(0);

    let mut pos = pos;
    while pos > 0 {
        bit_pos -= 1;
        pos -= 1;
        let bit = state.viterbi_history[pos] & (1 << (state_idx >> 4));
        state_idx >>= 1;
        if bit != 0 {
            state_idx |= 0x80;
            if bit_pos / 8 < out.len() {
                out[bit_pos / 8] |= 1 << (7 - (bit_pos % 8));
            }
        }
    }

    state.prev_metrics.iter().copied().min().unwrap_or(0)
}
fn viterbi_reset() {
    let mut state = state().lock().expect("viterbi mutex poisoned");
    state.viterbi_history.fill(0);
    state.curr_metrics.fill(0);
    state.prev_metrics.fill(0);
    state.curr_metrics_data.fill(0);
    state.prev_metrics_data.fill(0);
}
