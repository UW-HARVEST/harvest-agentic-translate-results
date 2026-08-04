use Megalania::lzma_packet::{LZMAPacket, LZMAPacketType};
use Megalania::lzma_state::{LZMAProperties, LZMAState, make_lzma_probability_model};
use Megalania::packet_enumerator::PacketEnumerator;
use Megalania::packet_slab::PacketSlab;
use Megalania::top_k_packet_finder::TopKPacketFinder;

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
fn test_count_at_pos_0_filters_literal() {
    // C output: count at pos 0 = 0 (because the only packet emitted is literal,
    // and the slab is full of literals so it gets filtered out as duplicate).
    let data: &[u8] = b"abcabcabcab";
    let pe = PacketEnumerator::new(data);
    let mut finder = TopKPacketFinder::new(5, &pe);
    let s = make_state(data);
    let mut slab = PacketSlab::new(data.len());
    finder.find(&s, slab.packets());
    assert_eq!(finder.count(), 0);
    assert!(finder.pop().is_none());
}

#[test]
fn test_count_at_pos_3_is_5() {
    // C output: count at pos 3 = 5 (matches of dist=2, len=4..8).
    let data: &[u8] = b"abcabcabcab";
    let pe = PacketEnumerator::new(data);
    let mut finder = TopKPacketFinder::new(5, &pe);
    let mut s = make_state(data);
    s.position = 3;
    let mut slab = PacketSlab::new(data.len());
    finder.find(&s, slab.packets());
    assert_eq!(finder.count(), 5);

    // Pop all entries. The last entry popped should be the best (lowest cost).
    let mut popped: Vec<LZMAPacket> = vec![];
    while let Some(p) = finder.pop() {
        popped.push(p);
    }
    assert_eq!(popped.len(), 5);
    // All should be matches with dist=2 and len 4..=8 (the best 5 of dists 2-8).
    let mut lengths: Vec<u16> = popped.iter().map(|p| p.len).collect();
    lengths.sort();
    assert_eq!(lengths, vec![4, 5, 6, 7, 8]);
    // All should be Match type with dist=2
    for p in &popped {
        assert_eq!(p.packet_type, LZMAPacketType::Match);
        assert_eq!(p.dist, 2);
    }
    assert_eq!(finder.count(), 0);
}

#[test]
fn test_count_after_clear_when_re_finding() {
    // Calling find again should reset the heap to fresh entries.
    let data: &[u8] = b"abcabcabcab";
    let pe = PacketEnumerator::new(data);
    let mut finder = TopKPacketFinder::new(5, &pe);
    let mut s = make_state(data);
    let mut slab = PacketSlab::new(data.len());

    s.position = 3;
    finder.find(&s, slab.packets());
    assert_eq!(finder.count(), 5);

    // Now find at pos 0 (should be 0).
    s.position = 0;
    finder.find(&s, slab.packets());
    assert_eq!(finder.count(), 0);
}

fn main() {}
