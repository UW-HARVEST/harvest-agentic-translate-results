use Megalania::lzma_packet::{LZMAPacket, LZMAPacketType};
use Megalania::packet_slab::PacketSlab;

#[test]
fn test_new_initializes_with_literals() {
    let slab = PacketSlab::new(10);
    assert_eq!(slab.size(), 10);
    for i in 0..10 {
        assert_eq!(slab.packets[i].packet_type, LZMAPacketType::Literal);
        assert_eq!(slab.packets[i].dist, 0);
        assert_eq!(slab.packets[i].len, 1);
    }
}

#[test]
fn test_count_all_literals() {
    let slab = PacketSlab::new(10);
    // Each literal has len=1, so count should be data_size
    assert_eq!(slab.count(), 10);
}

#[test]
fn test_count_with_match_packets() {
    let mut slab = PacketSlab::new(10);
    // Replace first packet with a length-5 match
    slab.packets[0] = LZMAPacket::match_packet(2, 5);
    // Then literal packets at positions 5..9 (len=1 each)
    // Total: 1 + 5 = 6 packets
    assert_eq!(slab.count(), 6);
}

#[test]
fn test_packets_slice() {
    let mut slab = PacketSlab::new(5);
    let packets = slab.packets();
    assert_eq!(packets.len(), 5);
    packets[0] = LZMAPacket::short_rep_packet();
    assert_eq!(slab.packets[0].packet_type, LZMAPacketType::ShortRep);
}

#[test]
fn test_restore_packet() {
    let mut slab = PacketSlab::new(5);
    let new_pkt = LZMAPacket::match_packet(3, 7);
    slab.restore_packet(2, new_pkt);
    assert_eq!(slab.packets[2].packet_type, LZMAPacketType::Match);
    assert_eq!(slab.packets[2].dist, 3);
    assert_eq!(slab.packets[2].len, 7);
}

#[test]
fn test_size_zero() {
    let slab = PacketSlab::new(0);
    assert_eq!(slab.size(), 0);
    assert_eq!(slab.count(), 0);
}

#[test]
fn test_memory_usage_nonzero() {
    let usage = PacketSlab::memory_usage(100);
    assert!(usage > 0);
}

fn main() {}
