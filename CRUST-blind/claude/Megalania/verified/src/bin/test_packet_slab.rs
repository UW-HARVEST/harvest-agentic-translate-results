use Megalania::lzma_packet::{LZMAPacket, LZMAPacketType};
use Megalania::packet_slab::PacketSlab;

#[test]
fn test_memory_usage() {
    // memory_usage(0) returns sizeof(PacketSlab) + 0 packets.
    // C reports 16 (sizeof(PacketSlab) on 64-bit). The Rust impl returns
    // sizeof::<LZMAPacket>() * data_size + sizeof::<PacketSlab>(). The exact
    // numbers may differ, but the formula should grow linearly with data_size:
    let mu_0 = PacketSlab::memory_usage(0);
    let mu_100 = PacketSlab::memory_usage(100);
    let mu_200 = PacketSlab::memory_usage(200);
    assert_eq!(mu_100 - mu_0, 100 * std::mem::size_of::<LZMAPacket>());
    assert_eq!(mu_200 - mu_0, 200 * std::mem::size_of::<LZMAPacket>());
}

#[test]
fn test_new_size_and_init() {
    let mut slab = PacketSlab::new(10);
    assert_eq!(slab.size(), 10);
    // All packets should initially be literals.
    for p in slab.packets().iter() {
        assert_eq!(p.packet_type, LZMAPacketType::Literal);
        assert_eq!(p.dist, 0);
        assert_eq!(p.len, 1);
    }
    // Count: each literal has len=1, so the count traverses the whole slab.
    assert_eq!(slab.count(), 10);
}

#[test]
fn test_count_with_match() {
    let mut slab = PacketSlab::new(10);
    {
        let packets = slab.packets();
        packets[0] = LZMAPacket::match_packet(5, 3); // skips 3 positions
    }
    // 1 (match of len 3) + remaining 7 literal positions = 8 packets
    assert_eq!(slab.count(), 8);
}

#[test]
fn test_count_with_match_5() {
    let mut slab = PacketSlab::new(10);
    {
        let packets = slab.packets();
        packets[0] = LZMAPacket::match_packet(5, 5);
    }
    // 1 match (len 5) + 5 literals = 6 packets
    assert_eq!(slab.count(), 6);
}

#[test]
fn test_restore_packet() {
    let mut slab = PacketSlab::new(5);
    let new_packet = LZMAPacket::match_packet(3, 2);
    slab.restore_packet(2, new_packet);
    assert_eq!(slab.packets()[2].packet_type, LZMAPacketType::Match);
    assert_eq!(slab.packets()[2].dist, 3);
    assert_eq!(slab.packets()[2].len, 2);
}

fn main() {}
