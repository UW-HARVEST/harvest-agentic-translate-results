use crate::math::q_abs_diff;
use std::cell::RefCell;

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
    history: [u16; 244],
}

impl ViterbiState {
    fn new() -> Self {
        Self {
            prev_metrics: [0; NUM_STATES],
            curr_metrics: [0; NUM_STATES],
            prev_metrics_data: [0; NUM_STATES],
            curr_metrics_data: [0; NUM_STATES],
            history: [0; 244],
        }
    }
}

thread_local! {
    static VITERBI_STATE: RefCell<ViterbiState> = RefCell::new(ViterbiState::new());
}

pub fn viterbi_decode(out: &mut [u8], input: &[u16], len: u16) -> u32 {
    viterbi_reset();

    let mut pos = 0usize;
    let total = len as usize;
    let mut i = 0usize;
    while i < total {
        viterbi_decode_bit(input[i], input[i + 1], pos);
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

    viterbi_decode(out, &umsg[..u], u as u16) - ((u - in_len as usize) as u32 * 0x7FFF)
}
pub fn viterbi_decode_bit(s0: u16, s1: u16, pos: usize) {
    const COST_TABLE_0: [u16; 8] = [0, 0, 0, 0, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF];
    const COST_TABLE_1: [u16; 8] = [0, 0xFFFF, 0xFFFF, 0, 0, 0xFFFF, 0xFFFF, 0];

    VITERBI_STATE.with(|state| {
        let mut state = state.borrow_mut();

        for i in 0..(NUM_STATES / 2) {
            let metric =
                u32::from(q_abs_diff(COST_TABLE_0[i], s0)) + u32::from(q_abs_diff(COST_TABLE_1[i], s1));

            let m0 = state.prev_metrics[i] + metric;
            let m1 = state.prev_metrics[i + NUM_STATES / 2] + (0x1FFFE - metric);
            let m2 = state.prev_metrics[i] + (0x1FFFE - metric);
            let m3 = state.prev_metrics[i + NUM_STATES / 2] + metric;

            let i0 = 2 * i;
            let i1 = i0 + 1;

            if m0 >= m1 {
                state.history[pos] |= 1 << i0;
                state.curr_metrics[i0] = m1;
            } else {
                state.history[pos] &= !(1 << i0);
                state.curr_metrics[i0] = m0;
            }

            if m2 >= m3 {
                state.history[pos] |= 1 << i1;
                state.curr_metrics[i1] = m3;
            } else {
                state.history[pos] &= !(1 << i1);
                state.curr_metrics[i1] = m2;
            }
        }

        let tmp = state.curr_metrics;
        state.curr_metrics = state.prev_metrics;
        state.prev_metrics = tmp;
    });
}
fn viterbi_chainback(out: &mut [u8], pos: usize, len: u16) -> u32 {
    VITERBI_STATE.with(|state| {
        let state = state.borrow();
        let mut cur_state = 0u8;
        let mut pos = pos;
        let mut bit_pos = usize::from(len) + 4;

        let used = if len == 0 {
            0
        } else {
            ((usize::from(len) - 1) / 8) + 1
        };
        out[..used].fill(0);

        while pos > 0 {
            bit_pos -= 1;
            pos -= 1;
            let bit = state.history[pos] & (1 << (cur_state >> 4));
            cur_state >>= 1;
            if bit != 0 {
                cur_state |= 0x80;
                out[bit_pos / 8] |= 1 << (7 - (bit_pos % 8));
            }
        }

        state
            .prev_metrics
            .iter()
            .copied()
            .min()
            .unwrap_or(0)
    })
}
fn viterbi_reset() {
    VITERBI_STATE.with(|state| {
        *state.borrow_mut() = ViterbiState::new();
    });
}
