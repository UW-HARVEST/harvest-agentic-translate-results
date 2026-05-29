use Megalania::lzma_packet::LZMAPacketType;
use Megalania::lzma_state::{
    lzma_state_init, lzma_state_promote_distance_at, lzma_state_push_distance,
    lzma_state_update_ctx_state, make_lzma_probability_model, LZMAProperties, LZMAState,
    ALIGN_BITS, END_POS_MODEL_INDEX, HIGH_CODER_BITS, HIGH_CODER_SYMBOLS, LIT_PROBS_SIZE,
    LOW_CODER_BITS, LOW_CODER_SYMBOLS, MID_CODER_BITS, MID_CODER_SYMBOLS, NUM_FULL_DISTANCES,
    NUM_LEN_TO_POS_STATES, NUM_POS_BITS_MAX, NUM_STATES, POS_SLOT_BITS,
};
use Megalania::probability::PROB_INIT_VAL;

fn make_state<'a>(data: &'a [u8]) -> LZMAState<'a> {
    LZMAState {
        data,
        properties: LZMAProperties { lc: 0, lp: 0, pb: 0 },
        ctx_state: 0,
        dists: [0; 4],
        probs: make_lzma_probability_model(),
        position: 0,
    }
}

#[test]
fn test_constants() {
    assert_eq!(NUM_POS_BITS_MAX, 4);
    assert_eq!(LOW_CODER_BITS, 3);
    assert_eq!(LOW_CODER_SYMBOLS, 8);
    assert_eq!(MID_CODER_BITS, 3);
    assert_eq!(MID_CODER_SYMBOLS, 8);
    assert_eq!(HIGH_CODER_BITS, 8);
    assert_eq!(HIGH_CODER_SYMBOLS, 256);
    assert_eq!(NUM_LEN_TO_POS_STATES, 4);
    assert_eq!(POS_SLOT_BITS, 6);
    assert_eq!(ALIGN_BITS, 4);
    assert_eq!(END_POS_MODEL_INDEX, 14);
    assert_eq!(NUM_FULL_DISTANCES, 1 << 7);
    assert_eq!(NUM_STATES, 12);
    assert_eq!(LIT_PROBS_SIZE, 0x300);
}

#[test]
fn test_lzma_state_init_resets_fields() {
    let data: [u8; 5] = [1, 2, 3, 4, 5];
    let mut s = make_state(&data);
    s.position = 99;
    s.ctx_state = 7;
    s.dists = [9, 8, 7, 6];
    let new_data: [u8; 3] = [9, 9, 9];
    let new_props = LZMAProperties { lc: 1, lp: 2, pb: 3 };
    lzma_state_init(&mut s, &new_data, new_props);
    assert_eq!(s.position, 0);
    assert_eq!(s.ctx_state, 0);
    assert_eq!(s.dists, [0u32; 4]);
    assert_eq!(s.properties.lc, 1);
    assert_eq!(s.properties.lp, 2);
    assert_eq!(s.properties.pb, 3);
    assert_eq!(s.data.len(), 3);
    // Probability model should be reset
    for &p in s.probs.lit.iter() {
        assert_eq!(p, PROB_INIT_VAL);
    }
}

#[test]
fn test_make_lzma_probability_model_init_vals() {
    let model = make_lzma_probability_model();
    assert_eq!(model.lit.len(), LIT_PROBS_SIZE);
    for &p in model.lit.iter() {
        assert_eq!(p, PROB_INIT_VAL);
    }
    assert_eq!(model.len.choice_1, PROB_INIT_VAL);
    assert_eq!(model.len.choice_2, PROB_INIT_VAL);
    for row in &model.len.low_coder {
        for &p in row.iter() {
            assert_eq!(p, PROB_INIT_VAL);
        }
    }
    for row in &model.len.mid_coder {
        for &p in row.iter() {
            assert_eq!(p, PROB_INIT_VAL);
        }
    }
    for &p in model.len.high_coder.iter() {
        assert_eq!(p, PROB_INIT_VAL);
    }
    assert_eq!(model.dist.pos_coder.len(), 1 + NUM_FULL_DISTANCES - END_POS_MODEL_INDEX);
    for &p in &model.dist.pos_coder {
        assert_eq!(p, PROB_INIT_VAL);
    }
    for &p in model.dist.align_coder.iter() {
        assert_eq!(p, PROB_INIT_VAL);
    }
    for &p in model.ctx_state.is_match.iter() {
        assert_eq!(p, PROB_INIT_VAL);
    }
    for &p in model.ctx_state.is_rep.iter() {
        assert_eq!(p, PROB_INIT_VAL);
    }
    for &p in model.ctx_state.is_rep_g0.iter() {
        assert_eq!(p, PROB_INIT_VAL);
    }
    for &p in model.ctx_state.is_rep_g1.iter() {
        assert_eq!(p, PROB_INIT_VAL);
    }
    for &p in model.ctx_state.is_rep_g2.iter() {
        assert_eq!(p, PROB_INIT_VAL);
    }
    for &p in model.ctx_state.is_rep0_long.iter() {
        assert_eq!(p, PROB_INIT_VAL);
    }
}

#[test]
fn test_push_distance() {
    let data = [0u8; 1];
    let mut s = make_state(&data);
    s.dists = [10, 20, 30, 40];
    lzma_state_push_distance(&mut s, 99);
    assert_eq!(s.dists, [99, 10, 20, 30]);
}

#[test]
fn test_promote_distance_at_0() {
    let data = [0u8; 1];
    let mut s = make_state(&data);
    s.dists = [10, 20, 30, 40];
    lzma_state_promote_distance_at(&mut s, 0);
    assert_eq!(s.dists, [10, 20, 30, 40]);
}

#[test]
fn test_promote_distance_at_1() {
    let data = [0u8; 1];
    let mut s = make_state(&data);
    s.dists = [10, 20, 30, 40];
    lzma_state_promote_distance_at(&mut s, 1);
    assert_eq!(s.dists, [20, 10, 30, 40]);
}

#[test]
fn test_promote_distance_at_2() {
    let data = [0u8; 1];
    let mut s = make_state(&data);
    s.dists = [10, 20, 30, 40];
    lzma_state_promote_distance_at(&mut s, 2);
    assert_eq!(s.dists, [30, 10, 20, 40]);
}

#[test]
fn test_promote_distance_at_3() {
    let data = [0u8; 1];
    let mut s = make_state(&data);
    s.dists = [10, 20, 30, 40];
    lzma_state_promote_distance_at(&mut s, 3);
    assert_eq!(s.dists, [40, 10, 20, 30]);
}

// Per the C output:
// LITERAL transitions
#[test]
fn test_update_ctx_state_literal_table() {
    let expected = [0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 4, 5];
    let data = [0u8; 1];
    for init in 0..12u8 {
        let mut s = make_state(&data);
        s.ctx_state = init;
        lzma_state_update_ctx_state(&mut s, LZMAPacketType::Literal as u32);
        assert_eq!(s.ctx_state, expected[init as usize], "init={}", init);
    }
}

#[test]
fn test_update_ctx_state_match_table() {
    let expected = [7, 7, 7, 7, 7, 7, 7, 10, 10, 10, 10, 10];
    let data = [0u8; 1];
    for init in 0..12u8 {
        let mut s = make_state(&data);
        s.ctx_state = init;
        lzma_state_update_ctx_state(&mut s, LZMAPacketType::Match as u32);
        assert_eq!(s.ctx_state, expected[init as usize], "init={}", init);
    }
}

#[test]
fn test_update_ctx_state_short_rep_table() {
    let expected = [9, 9, 9, 9, 9, 9, 9, 11, 11, 11, 11, 11];
    let data = [0u8; 1];
    for init in 0..12u8 {
        let mut s = make_state(&data);
        s.ctx_state = init;
        lzma_state_update_ctx_state(&mut s, LZMAPacketType::ShortRep as u32);
        assert_eq!(s.ctx_state, expected[init as usize], "init={}", init);
    }
}

#[test]
fn test_update_ctx_state_long_rep_table() {
    let expected = [8, 8, 8, 8, 8, 8, 8, 11, 11, 11, 11, 11];
    let data = [0u8; 1];
    for init in 0..12u8 {
        let mut s = make_state(&data);
        s.ctx_state = init;
        lzma_state_update_ctx_state(&mut s, LZMAPacketType::LongRep as u32);
        assert_eq!(s.ctx_state, expected[init as usize], "init={}", init);
    }
}

fn main() {}
