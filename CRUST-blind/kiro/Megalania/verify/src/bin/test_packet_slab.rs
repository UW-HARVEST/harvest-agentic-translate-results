use Megalania::lzma_packet::{LZMAPacket, LZMAPacketType};
use Megalania::packet_slab::PacketSlab;

#[test]
fn test_new_slab() {
    let slab = PacketSlab::new(10);
    assert_eq!(slab.size(), 10);
    assert_eq!(slab.count(), 10);
}

#[test]
fn test_all_literals() {
    let mut slab = PacketSlab::new(10);
    for p in slab.packets().iter() {
        assert_eq!(p.packet_type, LZMAPacketType::Literal);
        assert_eq!(p.len, 1);
        assert_eq!(p.dist, 0);
    }
}

#[test]
fn test_count_after_match() {
    // C ground truth: after match(5,3) at 0, count=8
    let mut slab = PacketSlab::new(10);
    slab.packets()[0] = LZMAPacket::match_packet(5, 3);
    assert_eq!(slab.count(), 8);
}

#[test]
fn test_restore_packet() {
    let mut slab = PacketSlab::new(5);
    let orig = slab.packets()[0];
    slab.packets()[0] = LZMAPacket::short_rep_packet();
    assert_eq!(slab.packets()[0].packet_type, LZMAPacketType::ShortRep);
    slab.restore_packet(0, orig);
    assert_eq!(slab.packets()[0].packet_type, LZMAPacketType::Literal);
}

fn main() {}
