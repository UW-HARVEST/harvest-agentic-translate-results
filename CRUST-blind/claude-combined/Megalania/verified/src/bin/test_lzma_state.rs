use Megalania::lzma_state::{
    lzma_state_init, lzma_state_promote_distance_at, lzma_state_push_distance,
    lzma_state_update_ctx_state, LZMAProperties, LZMAState,
};
use Megalania::probability::PROB_INIT_VAL;

fn make_state<'a>(data: &'a [u8]) -> LZMAState<'a> {
    let props = LZMAProperties { lc: 0, lp: 0, pb: 0 };
    LZMAState::new(data, props)
}

#[test]
fn test_state_init_resets_fields() {
    let data: [u8; 4] = [1, 2, 3, 4];
    let mut s = make_state(&data);
    s.ctx_state = 5;
    s.dists = [10, 20, 30, 40];
    s.position = 9;
    s.probs.lit[0] = 999;
    let new_data: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
    let new_props = LZMAProperties { lc: 1, lp: 2, pb: 3 };
    lzma_state_init(&mut s, &new_data, new_props);
    assert_eq!(s.ctx_state, 0);
    assert_eq!(s.dists, [0, 0, 0, 0]);
    assert_eq!(s.position, 0);
    assert_eq!(s.properties.lc, 1);
    assert_eq!(s.properties.lp, 2);
    assert_eq!(s.properties.pb, 3);
    assert_eq!(s.probs.lit[0], PROB_INIT_VAL);
    assert_eq!(s.probs.lit[100], PROB_INIT_VAL);
    assert_eq!(s.probs.len.choice_1, PROB_INIT_VAL);
    assert_eq!(s.probs.dist.align_coder[0], PROB_INIT_VAL);
    assert_eq!(s.probs.ctx_state.is_match[0], PROB_INIT_VAL);
}

#[test]
fn test_ctx_state_literal_transitions() {
    let data: [u8; 4] = [0; 4];
    let cases = [
        (0, 1, 0),
        (3, 1, 0),
        (4, 1, 1),
        (6, 1, 3),
        (9, 1, 6),
        (10, 1, 4),
        (11, 1, 5),
    ];
    for (initial, ptype, expected) in cases.iter() {
        let mut s = make_state(&data);
        s.ctx_state = *initial;
        lzma_state_update_ctx_state(&mut s, *ptype as u32);
        assert_eq!(s.ctx_state, *expected, "literal: ctx={} type={}", initial, ptype);
    }
}

#[test]
fn test_ctx_state_match_transitions() {
    let data: [u8; 4] = [0; 4];
    let cases = [(0, 7), (3, 7), (6, 7), (7, 10), (9, 10), (11, 10)];
    for (initial, expected) in cases.iter() {
        let mut s = make_state(&data);
        s.ctx_state = *initial;
        lzma_state_update_ctx_state(&mut s, 2);
        assert_eq!(s.ctx_state, *expected, "match: ctx={}", initial);
    }
}

#[test]
fn test_ctx_state_short_rep_transitions() {
    let data: [u8; 4] = [0; 4];
    let cases = [(0, 9), (3, 9), (6, 9), (7, 11), (9, 11), (11, 11)];
    for (initial, expected) in cases.iter() {
        let mut s = make_state(&data);
        s.ctx_state = *initial;
        lzma_state_update_ctx_state(&mut s, 3);
        assert_eq!(s.ctx_state, *expected, "short_rep: ctx={}", initial);
    }
}

#[test]
fn test_ctx_state_long_rep_transitions() {
    let data: [u8; 4] = [0; 4];
    let cases = [(0, 8), (3, 8), (6, 8), (7, 11), (9, 11), (11, 11)];
    for (initial, expected) in cases.iter() {
        let mut s = make_state(&data);
        s.ctx_state = *initial;
        lzma_state_update_ctx_state(&mut s, 4);
        assert_eq!(s.ctx_state, *expected, "long_rep: ctx={}", initial);
    }
}

#[test]
fn test_push_distance() {
    let data: [u8; 4] = [0; 4];
    let mut s = make_state(&data);
    lzma_state_push_distance(&mut s, 1);
    assert_eq!(s.dists, [1, 0, 0, 0]);
    lzma_state_push_distance(&mut s, 2);
    assert_eq!(s.dists, [2, 1, 0, 0]);
    lzma_state_push_distance(&mut s, 3);
    assert_eq!(s.dists, [3, 2, 1, 0]);
    lzma_state_push_distance(&mut s, 4);
    assert_eq!(s.dists, [4, 3, 2, 1]);
    lzma_state_push_distance(&mut s, 5);
    assert_eq!(s.dists, [5, 4, 3, 2]);
}

#[test]
fn test_promote_distance_at_0() {
    let data: [u8; 4] = [0; 4];
    let mut s = make_state(&data);
    s.dists = [10, 20, 30, 40];
    lzma_state_promote_distance_at(&mut s, 0);
    assert_eq!(s.dists, [10, 20, 30, 40]);
}

#[test]
fn test_promote_distance_at_1() {
    let data: [u8; 4] = [0; 4];
    let mut s = make_state(&data);
    s.dists = [10, 20, 30, 40];
    lzma_state_promote_distance_at(&mut s, 1);
    assert_eq!(s.dists, [20, 10, 30, 40]);
}

#[test]
fn test_promote_distance_at_2() {
    let data: [u8; 4] = [0; 4];
    let mut s = make_state(&data);
    s.dists = [10, 20, 30, 40];
    lzma_state_promote_distance_at(&mut s, 2);
    assert_eq!(s.dists, [30, 10, 20, 40]);
}

#[test]
fn test_promote_distance_at_3() {
    let data: [u8; 4] = [0; 4];
    let mut s = make_state(&data);
    s.dists = [10, 20, 30, 40];
    lzma_state_promote_distance_at(&mut s, 3);
    assert_eq!(s.dists, [40, 10, 20, 30]);
}

fn main() {}
