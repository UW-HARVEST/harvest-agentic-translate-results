use std::cell::RefCell;
use Megalania::lzma_packet::{LZMAPacket, LZMAPacketType};
use Megalania::lzma_state::{LZMAProperties, LZMAState, make_lzma_probability_model};
use Megalania::packet_enumerator::PacketEnumerator;

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
fn test_position_zero_only_emits_literal() {
    // For "abcabcab", at pos 0, only literal is emitted.
    let data: &[u8] = b"abcabcab";
    let pe = PacketEnumerator::new(data);
    let s = make_state(data);
    let result = RefCell::new(Vec::<LZMAPacket>::new());
    pe.for_each(&s, |_state, packet| result.borrow_mut().push(packet));
    let r = result.into_inner();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].packet_type, LZMAPacketType::Literal);
    assert_eq!(r[0].dist, 0);
    assert_eq!(r[0].len, 1);
}

#[test]
fn test_position_3() {
    // C output:
    // pos 3: count=5
    //   type=1 dist=0 len=1   (LITERAL)
    //   type=2 dist=2 len=2   (MATCH)
    //   type=2 dist=2 len=3   (MATCH)
    //   type=2 dist=2 len=4   (MATCH)
    //   type=2 dist=2 len=5   (MATCH)
    let data: &[u8] = b"abcabcab";
    let pe = PacketEnumerator::new(data);
    let mut s = make_state(data);
    s.position = 3;

    let result = RefCell::new(Vec::<LZMAPacket>::new());
    pe.for_each(&s, |_state, packet| result.borrow_mut().push(packet));
    let r = result.into_inner();
    assert_eq!(r.len(), 5);
    assert_eq!(r[0].packet_type, LZMAPacketType::Literal);
    assert_eq!(r[0].dist, 0);
    assert_eq!(r[0].len, 1);

    assert_eq!(r[1].packet_type, LZMAPacketType::Match);
    assert_eq!(r[1].dist, 2);
    assert_eq!(r[1].len, 2);

    assert_eq!(r[2].packet_type, LZMAPacketType::Match);
    assert_eq!(r[2].dist, 2);
    assert_eq!(r[2].len, 3);

    assert_eq!(r[3].packet_type, LZMAPacketType::Match);
    assert_eq!(r[3].dist, 2);
    assert_eq!(r[3].len, 4);

    assert_eq!(r[4].packet_type, LZMAPacketType::Match);
    assert_eq!(r[4].dist, 2);
    assert_eq!(r[4].len, 5);
}

#[test]
fn test_position_6() {
    // C output:
    // pos 6: count=3
    //   type=1 dist=0 len=1
    //   type=2 dist=5 len=2
    //   type=2 dist=2 len=2
    let data: &[u8] = b"abcabcab";
    let pe = PacketEnumerator::new(data);
    let mut s = make_state(data);
    s.position = 6;

    let result = RefCell::new(Vec::<LZMAPacket>::new());
    pe.for_each(&s, |_state, packet| result.borrow_mut().push(packet));
    let r = result.into_inner();
    assert_eq!(r.len(), 3);
    assert_eq!(r[0].packet_type, LZMAPacketType::Literal);
    assert_eq!(r[0].dist, 0);
    assert_eq!(r[0].len, 1);

    assert_eq!(r[1].packet_type, LZMAPacketType::Match);
    assert_eq!(r[1].dist, 5);
    assert_eq!(r[1].len, 2);

    assert_eq!(r[2].packet_type, LZMAPacketType::Match);
    assert_eq!(r[2].dist, 2);
    assert_eq!(r[2].len, 2);
}

#[test]
fn test_short_rep_when_dists0_matches() {
    // If dists[0] is set up such that data[pos] == data[pos - dists[0] - 1],
    // a short_rep packet should be emitted.
    let data: &[u8] = b"abab";
    let pe = PacketEnumerator::new(data);
    let mut s = make_state(data);
    s.position = 2;
    s.dists[0] = 1; // pos - dists[0] - 1 = 0; data[2]='a', data[0]='a' - matches!

    let result = RefCell::new(Vec::<LZMAPacket>::new());
    pe.for_each(&s, |_state, packet| result.borrow_mut().push(packet));
    let r = result.into_inner();
    // Should contain literal + short_rep + matches
    assert!(r.iter().any(|p| p.packet_type == LZMAPacketType::Literal));
    assert!(r.iter().any(|p| p.packet_type == LZMAPacketType::ShortRep));
    // dist 1 in dists[0] -> a long_rep (i=0) since dist 1 == dists[0]
    let has_long_rep = r.iter().any(|p| p.packet_type == LZMAPacketType::LongRep);
    assert!(has_long_rep, "should produce a long_rep packet");
}

#[test]
fn test_memory_usage_grows() {
    let m0 = PacketEnumerator::memory_usage(0);
    let m100 = PacketEnumerator::memory_usage(100);
    assert!(m100 > m0);
}

fn main() {}
