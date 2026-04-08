use Megalania::lzma_packet::{LZMAPacket, LZMAPacketType};
use Megalania::packet_slab::PacketSlab;
use Megalania::packet_slab_undo_stack::{PacketSlabUndo, PacketSlabUndoStack, UNDO_STACK_SIZE};

#[test]
fn test_undo_stack_size_constant() {
    assert_eq!(UNDO_STACK_SIZE, 16);
}

#[test]
fn test_new_stack() {
    let stack = PacketSlabUndoStack::new();
    assert_eq!(stack.count(), 0);
}

#[test]
fn test_insert_and_count() {
    let mut stack = PacketSlabUndoStack::new();
    for i in 0..20 {
        stack.insert(PacketSlabUndo {
            position: i,
            old_packet: LZMAPacket::literal_packet(),
        });
    }
    // C ground truth: count=20
    assert_eq!(stack.count(), 20);
}

#[test]
fn test_apply_restores_packets() {
    // C ground truth: change 20 packets to short_rep, apply undo -> all back to literal
    let mut slab = PacketSlab::new(32);
    let mut stack = PacketSlabUndoStack::new();

    for i in 0..20 {
        let old = slab.packets()[i];
        stack.insert(PacketSlabUndo {
            position: i,
            old_packet: old,
        });
        slab.packets()[i] = LZMAPacket::short_rep_packet();
    }

    // Verify they're short_rep before undo
    assert_eq!(slab.packets()[0].packet_type, LZMAPacketType::ShortRep);
    assert_eq!(slab.packets()[19].packet_type, LZMAPacketType::ShortRep);

    stack.apply(&mut slab);

    // After apply, all should be literal again
    assert_eq!(slab.packets()[0].packet_type, LZMAPacketType::Literal);
    assert_eq!(slab.packets()[19].packet_type, LZMAPacketType::Literal);
    assert_eq!(stack.count(), 0);
}

#[test]
fn test_apply_with_overflow() {
    // Test with more than UNDO_STACK_SIZE (16) entries to exercise heap path
    let mut slab = PacketSlab::new(1024);
    let mut stack = PacketSlabUndoStack::new();

    for i in 0..1024 {
        let old = slab.packets()[i];
        stack.insert(PacketSlabUndo {
            position: i,
            old_packet: old,
        });
        slab.packets()[i] = LZMAPacket::short_rep_packet();
    }
    assert_eq!(stack.count(), 1024);

    stack.apply(&mut slab);
    for i in 0..1024 {
        assert_eq!(slab.packets()[i].packet_type, LZMAPacketType::Literal);
    }
    assert_eq!(stack.count(), 0);
}

fn main() {}
