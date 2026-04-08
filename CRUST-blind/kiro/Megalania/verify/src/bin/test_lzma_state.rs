use Megalania::lzma_state::*;
use Megalania::lzma_packet::LZMAPacketType;

#[test]
fn test_init() {
    let data = [0x41u8, 0x42, 0x43, 0x44, 0x45];
    let props = LZMAProperties { lc: 3, lp: 0, pb: 2 };
    let state = LZMAState::new(&data, props);
    assert_eq!(state.ctx_state, 0);
    assert_eq!(state.position, 0);
    assert_eq!(state.dists, [0, 0, 0, 0]);
    assert_eq!(state.data.len(), 5);
    assert_eq!(state.properties.lc, 3);
    assert_eq!(state.properties.lp, 0);
    assert_eq!(state.properties.pb, 2);
}

#[test]
fn test_ctx_state_sequence() {
    // C ground truth: 0 -> LITERAL -> 0 -> MATCH -> 7 -> SHORT_REP -> 11 -> LONG_REP -> 11 -> LITERAL -> 5
    let data = [0x41u8];
    let props = LZMAProperties { lc: 3, lp: 0, pb: 2 };
    let mut state = LZMAState::new(&data, props);

    lzma_state_update_ctx_state(&mut state, LZMAPacketType::Literal as u32);
    assert_eq!(state.ctx_state, 0);
    lzma_state_update_ctx_state(&mut state, LZMAPacketType::Match as u32);
    assert_eq!(state.ctx_state, 7);
    lzma_state_update_ctx_state(&mut state, LZMAPacketType::ShortRep as u32);
    assert_eq!(state.ctx_state, 11);
    lzma_state_update_ctx_state(&mut state, LZMAPacketType::LongRep as u32);
    assert_eq!(state.ctx_state, 11);
    lzma_state_update_ctx_state(&mut state, LZMAPacketType::Literal as u32);
    assert_eq!(state.ctx_state, 5);
}

#[test]
fn test_full_transition_table() {
    // Ground truth from C: all 12 states x 4 packet types
    let expected: [[u8; 4]; 12] = [
        [0, 7, 9, 8],   // s=0: LIT,MATCH,SREP,LREP
        [0, 7, 9, 8],   // s=1
        [0, 7, 9, 8],   // s=2
        [0, 7, 9, 8],   // s=3
        [1, 7, 9, 8],   // s=4
        [2, 7, 9, 8],   // s=5
        [3, 7, 9, 8],   // s=6
        [4, 10, 11, 11], // s=7
        [5, 10, 11, 11], // s=8
        [6, 10, 11, 11], // s=9
        [4, 10, 11, 11], // s=10
        [5, 10, 11, 11], // s=11
    ];
    let types = [
        LZMAPacketType::Literal as u32,
        LZMAPacketType::Match as u32,
        LZMAPacketType::ShortRep as u32,
        LZMAPacketType::LongRep as u32,
    ];
    let data = [0u8];
    let props = LZMAProperties { lc: 3, lp: 0, pb: 2 };
    for s in 0..12u8 {
        for (ti, &t) in types.iter().enumerate() {
            let mut state = LZMAState::new(&data, props);
            state.ctx_state = s;
            lzma_state_update_ctx_state(&mut state, t);
            assert_eq!(state.ctx_state, expected[s as usize][ti],
                "s={} t={} expected {} got {}", s, t, expected[s as usize][ti], state.ctx_state);
        }
    }
}

#[test]
fn test_push_distance() {
    let data = [0u8];
    let props = LZMAProperties { lc: 3, lp: 0, pb: 2 };
    let mut state = LZMAState::new(&data, props);

    lzma_state_push_distance(&mut state, 100);
    lzma_state_push_distance(&mut state, 200);
    lzma_state_push_distance(&mut state, 300);
    lzma_state_push_distance(&mut state, 400);
    assert_eq!(state.dists, [400, 300, 200, 100]);

    lzma_state_push_distance(&mut state, 500);
    assert_eq!(state.dists, [500, 400, 300, 200]);
}

#[test]
fn test_promote_distance() {
    let data = [0u8];
    let props = LZMAProperties { lc: 3, lp: 0, pb: 2 };
    let mut state = LZMAState::new(&data, props);

    lzma_state_push_distance(&mut state, 100);
    lzma_state_push_distance(&mut state, 200);
    lzma_state_push_distance(&mut state, 300);
    lzma_state_push_distance(&mut state, 400);
    lzma_state_push_distance(&mut state, 500);
    // dists = [500, 400, 300, 200]

    lzma_state_promote_distance_at(&mut state, 2);
    assert_eq!(state.dists, [300, 500, 400, 200]);

    lzma_state_promote_distance_at(&mut state, 0);
    assert_eq!(state.dists, [300, 500, 400, 200]);

    lzma_state_promote_distance_at(&mut state, 3);
    assert_eq!(state.dists, [200, 300, 500, 400]);
}

#[test]
fn test_lzma_state_init_fn() {
    let data = [0x41u8, 0x42, 0x43];
    let props = LZMAProperties { lc: 3, lp: 0, pb: 2 };
    let mut state = LZMAState::new(&[0u8], props);
    lzma_state_init(&mut state, &data, props);
    assert_eq!(state.ctx_state, 0);
    assert_eq!(state.position, 0);
    assert_eq!(state.dists, [0, 0, 0, 0]);
    assert_eq!(state.data.len(), 3);
}

fn main() {}
