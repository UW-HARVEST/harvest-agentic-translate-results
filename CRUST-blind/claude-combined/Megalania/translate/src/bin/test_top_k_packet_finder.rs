use Megalania::lzma_packet::{LZMAPacket, LZMAPacketType};
use Megalania::lzma_state::{LZMAProperties, LZMAState};
use Megalania::packet_enumerator::PacketEnumerator;
use Megalania::packet_slab::PacketSlab;
use Megalania::top_k_packet_finder::TopKPacketFinder;

fn collect_pops(
    data: &[u8],
    pos: usize,
    k: usize,
) -> Vec<LZMAPacket> {
    let props = LZMAProperties { lc: 0, lp: 0, pb: 0 };
    let mut s = LZMAState::new(data, props);
    s.position = pos;

    let pe = PacketEnumerator::new(data);
    let mut finder = TopKPacketFinder::new(k, &pe);
    let mut slab = PacketSlab::new(data.len());
    finder.find(&s, slab.packets());
    let mut pops = Vec::new();
    while let Some(p) = finder.pop() {
        pops.push(p);
    }
    pops
}

#[test]
fn test_position_0_skips_literal() {
    // C: count=0 when slab packets are all literals (matches the literal packet enumerator emits)
    let data = b"hello hello";
    let pops = collect_pops(data, 0, 20);
    assert_eq!(pops.len(), 0);
}

#[test]
fn test_position_6_match_packets() {
    // C: count=4
    // pop order: type=2 dist=5 len=2, then 3, 4, 5
    let data = b"hello hello";
    let pops = collect_pops(data, 6, 20);
    assert_eq!(pops.len(), 4);
    // All should be Match packets with dist=5
    for p in &pops {
        assert_eq!(p.packet_type, LZMAPacketType::Match);
        assert_eq!(p.dist, 5);
    }
    // Lengths in pop order should be 2, 3, 4, 5
    let lens: Vec<u16> = pops.iter().map(|p| p.len).collect();
    assert_eq!(lens, vec![2, 3, 4, 5]);
}

#[test]
fn test_aaaaa_position_2() {
    // C: count=13 with specific order
    let data = b"aaaaa";
    let pops = collect_pops(data, 2, 20);
    assert_eq!(pops.len(), 13);

    // Verify the pop order matches the C output exactly:
    // type=2 dist=1 len=2
    // type=2 dist=0 len=2
    // type=4 dist=3 len=2
    // type=4 dist=2 len=2
    // type=3 dist=0 len=1
    // type=2 dist=0 len=3
    // type=4 dist=0 len=2
    // type=2 dist=1 len=3
    // type=4 dist=1 len=2
    // type=4 dist=2 len=3
    // type=4 dist=3 len=3
    // type=4 dist=1 len=3
    // type=4 dist=0 len=3
    let expected: Vec<(LZMAPacketType, u32, u16)> = vec![
        (LZMAPacketType::Match, 1, 2),
        (LZMAPacketType::Match, 0, 2),
        (LZMAPacketType::LongRep, 3, 2),
        (LZMAPacketType::LongRep, 2, 2),
        (LZMAPacketType::ShortRep, 0, 1),
        (LZMAPacketType::Match, 0, 3),
        (LZMAPacketType::LongRep, 0, 2),
        (LZMAPacketType::Match, 1, 3),
        (LZMAPacketType::LongRep, 1, 2),
        (LZMAPacketType::LongRep, 2, 3),
        (LZMAPacketType::LongRep, 3, 3),
        (LZMAPacketType::LongRep, 1, 3),
        (LZMAPacketType::LongRep, 0, 3),
    ];
    for (i, (ptype, dist, len)) in expected.iter().enumerate() {
        assert_eq!(pops[i].packet_type, *ptype, "i={}", i);
        assert_eq!(pops[i].dist, *dist, "i={}", i);
        assert_eq!(pops[i].len, *len, "i={}", i);
    }
}

#[test]
fn test_count_zero_after_initialization() {
    let data = b"hello hello";
    let pe = PacketEnumerator::new(data);
    let finder = TopKPacketFinder::new(20, &pe);
    assert_eq!(finder.count(), 0);
}

#[test]
fn test_pop_empty_returns_none() {
    let data = b"hello";
    let pe = PacketEnumerator::new(data);
    let mut finder = TopKPacketFinder::new(5, &pe);
    assert_eq!(finder.pop(), None);
}

fn main() {}
