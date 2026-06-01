use Megalania::lzma_packet::{LZMAPacket, LZMAPacketType};
use Megalania::packet_slab::PacketSlab;
use Megalania::packet_slab_undo_stack::{PacketSlabUndo, PacketSlabUndoStack, UNDO_STACK_SIZE};

#[test]
fn test_new_initial_state() {
    let s = PacketSlabUndoStack::new();
    assert_eq!(s.count(), 0);
    assert_eq!(s.total_count, 0);
    assert_eq!(s.count, 0);
    for slot in s.stack.iter() {
        assert!(slot.is_none());
    }
    assert_eq!(s.extra.len(), 0);
}

#[test]
fn test_insert_within_stack_limit() {
    let mut s = PacketSlabUndoStack::new();
    for i in 0..10 {
        let undo = PacketSlabUndo {
            position: i,
            old_packet: LZMAPacket::literal_packet(),
        };
        s.insert(undo);
    }
    assert_eq!(s.count, 10);
    assert_eq!(s.total_count, 10);
    assert_eq!(s.count(), 10);
    assert_eq!(s.extra.len(), 0);
    // Check positions are stored
    for i in 0..10 {
        let entry = s.stack[i].as_ref().expect("entry should exist");
        assert_eq!(entry.position, i);
    }
}

#[test]
fn test_insert_overflows_to_extra() {
    let mut s = PacketSlabUndoStack::new();
    for i in 0..(UNDO_STACK_SIZE + 5) {
        s.insert(PacketSlabUndo {
            position: i,
            old_packet: LZMAPacket::literal_packet(),
        });
    }
    assert_eq!(s.count, UNDO_STACK_SIZE);
    assert_eq!(s.total_count, UNDO_STACK_SIZE + 5);
    assert_eq!(s.extra.len(), 5);
    assert_eq!(s.count(), UNDO_STACK_SIZE + 5);
}

#[test]
fn test_apply_restores_packets() {
    // Following the C test packet_slab_undo_stack_test_apply
    let slab_size = 1024;
    let mut slab = PacketSlab::new(slab_size);
    let mut undo_stack = PacketSlabUndoStack::new();

    // Change all packets to short reps
    for i in 0..slab_size {
        let old = slab.packets()[i];
        slab.packets()[i] = LZMAPacket::short_rep_packet();
        undo_stack.insert(PacketSlabUndo {
            position: i,
            old_packet: old,
        });
    }
    // Apply undo
    undo_stack.apply(&mut slab);
    for i in 0..slab_size {
        assert_eq!(slab.packets[i].packet_type, LZMAPacketType::Literal);
    }
    assert_eq!(undo_stack.count, 0);
    assert_eq!(undo_stack.total_count, 0);
}

#[test]
fn test_apply_clears_count() {
    let mut slab = PacketSlab::new(20);
    let mut undo_stack = PacketSlabUndoStack::new();
    for i in 0..5 {
        let old = slab.packets()[i];
        slab.packets()[i] = LZMAPacket::short_rep_packet();
        undo_stack.insert(PacketSlabUndo {
            position: i,
            old_packet: old,
        });
    }
    assert_eq!(undo_stack.count(), 5);
    undo_stack.apply(&mut slab);
    assert_eq!(undo_stack.count(), 0);
    for i in 0..5 {
        assert_eq!(slab.packets[i].packet_type, LZMAPacketType::Literal);
    }
}

#[test]
fn test_undo_stack_size_is_16() {
    assert_eq!(UNDO_STACK_SIZE, 16);
}

fn main() {}
