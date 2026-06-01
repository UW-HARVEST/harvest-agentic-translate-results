use Megalania::lzma_packet::{LZMAPacket, LZMAPacketType};
use Megalania::lzma_state::{LZMAProperties, LZMAState};
use Megalania::packet_enumerator::PacketEnumerator;
use Megalania::packet_slab::PacketSlab;
use Megalania::packet_slab_neighbour::PacketSlabNeighbour;
use Megalania::packet_slab_undo_stack::PacketSlabUndo;
use Megalania::top_k_packet_finder::TopKPacketFinder;

#[test]
fn test_new_initial_state() {
    let data = vec![0u8; 16];
    let props = LZMAProperties { lc: 0, lp: 0, pb: 0 };
    let state = LZMAState::new(&data, props);
    let slab = PacketSlab::new(16);
    let n = PacketSlabNeighbour::new(slab, state);
    assert_eq!(n.perplexity, 0);
    assert_eq!(n.undo_count(), 0);
    // The slab inside should have all literal packets
    for p in n.slab.packets.iter() {
        assert_eq!(p.packet_type, LZMAPacketType::Literal);
    }
}

#[test]
fn test_undo_count_starts_zero() {
    let data = vec![0u8; 8];
    let props = LZMAProperties { lc: 0, lp: 0, pb: 0 };
    let state = LZMAState::new(&data, props);
    let slab = PacketSlab::new(8);
    let n = PacketSlabNeighbour::new(slab, state);
    assert_eq!(n.undo_count(), 0);
}

#[test]
fn test_undo_restores_packets() {
    // Without using `generate` (which involves randomness), we test by manually
    // mutating the slab through the undo stack.
    let data = vec![0u8; 8];
    let props = LZMAProperties { lc: 0, lp: 0, pb: 0 };
    let state = LZMAState::new(&data, props);
    let slab = PacketSlab::new(8);
    let mut n = PacketSlabNeighbour::new(slab, state);

    // Take note of the original packets
    for i in 0..8 {
        let old = n.slab.packets[i];
        n.slab.packets[i] = LZMAPacket::short_rep_packet();
        n.undo_stack.insert(PacketSlabUndo {
            position: i,
            old_packet: old,
        });
    }

    assert_eq!(n.undo_count(), 8);
    n.undo();
    // After undo, packets should be back to literals
    for i in 0..8 {
        assert_eq!(n.slab.packets[i].packet_type, LZMAPacketType::Literal);
    }
    assert_eq!(n.undo_count(), 0);
}

#[test]
fn test_generate_produces_some_changes_or_returns_false_safely() {
    // Just verify generate doesn't panic and returns a bool
    let data: Vec<u8> = (0..32u8).collect();
    let props = LZMAProperties { lc: 0, lp: 0, pb: 0 };
    let state = LZMAState::new(&data, props);
    let slab = PacketSlab::new(32);
    let mut n = PacketSlabNeighbour::new(slab, state.clone());

    let pe = PacketEnumerator::new(&data);
    let mut finder = TopKPacketFinder::new(20, &pe);
    let _result = n.generate(&mut finder);
    // The result type is bool; we don't crash.
}

fn main() {}
