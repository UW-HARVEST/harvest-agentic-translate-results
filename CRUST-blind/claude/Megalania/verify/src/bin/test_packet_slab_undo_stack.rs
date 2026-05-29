use Megalania::lzma_packet::{LZMAPacket, LZMAPacketType};
use Megalania::packet_slab::PacketSlab;
use Megalania::packet_slab_undo_stack::{PacketSlabUndo, PacketSlabUndoStack, UNDO_STACK_SIZE};

#[test]
fn test_constants() {
    assert_eq!(UNDO_STACK_SIZE, 16);
}

#[test]
fn test_initial_state() {
    let stack = PacketSlabUndoStack::new();
    assert_eq!(stack.count(), 0);
    assert_eq!(stack.total_count, 0);
    assert_eq!(stack.count, 0);
}

#[test]
fn test_insert_few() {
    let mut stack = PacketSlabUndoStack::new();
    let undo = PacketSlabUndo {
        position: 1,
        old_packet: LZMAPacket::literal_packet(),
    };
    for _ in 0..5 {
        stack.insert(undo.clone());
    }
    assert_eq!(stack.count(), 5);
}

#[test]
fn test_insert_many_apply() {
    // Replicate C test: SLAB_SIZE = 50, replace all packets with short_rep,
    // then apply. We use 50 to exercise the heap path (>16).
    let slab_size: usize = 50;
    let mut slab = PacketSlab::new(slab_size);
    let mut stack = PacketSlabUndoStack::new();

    {
        let packets = slab.packets();
        for i in 0..slab_size {
            let undo = PacketSlabUndo {
                position: i,
                old_packet: packets[i],
            };
            packets[i] = LZMAPacket::short_rep_packet();
            stack.insert(undo);
        }
    }
    assert_eq!(stack.count(), slab_size);
    // Verify that all packets are now SHORT_REP
    for p in slab.packets().iter() {
        assert_eq!(p.packet_type, LZMAPacketType::ShortRep);
    }

    stack.apply(&mut slab);
    assert_eq!(stack.count(), 0);
    assert_eq!(stack.count, 0);

    // After apply, all packets should be back to LITERAL
    for p in slab.packets().iter() {
        assert_eq!(p.packet_type, LZMAPacketType::Literal);
        assert_eq!(p.dist, 0);
        assert_eq!(p.len, 1);
    }
}

#[test]
fn test_insert_25_apply() {
    // Cross-checked with C: insert 25 in a 50-slot slab, then apply.
    let slab_size: usize = 50;
    let mut slab = PacketSlab::new(slab_size);
    let mut stack = PacketSlabUndoStack::new();

    {
        let packets = slab.packets();
        for i in 0..5 {
            let undo = PacketSlabUndo {
                position: i,
                old_packet: packets[i],
            };
            packets[i] = LZMAPacket::short_rep_packet();
            stack.insert(undo);
        }
    }
    assert_eq!(stack.count(), 5);

    {
        let packets = slab.packets();
        for i in 5..25 {
            let undo = PacketSlabUndo {
                position: i,
                old_packet: packets[i],
            };
            packets[i] = LZMAPacket::match_packet(2, 1);
            stack.insert(undo);
        }
    }
    assert_eq!(stack.count(), 25);

    stack.apply(&mut slab);
    assert_eq!(stack.count(), 0);
    for i in 0..25 {
        assert_eq!(slab.packets()[i].packet_type, LZMAPacketType::Literal);
    }
}

#[test]
fn test_insert_does_not_apply() {
    // Inserting does not undo: packets remain modified
    let slab_size: usize = 30;
    let mut slab = PacketSlab::new(slab_size);
    let mut stack = PacketSlabUndoStack::new();
    {
        let packets = slab.packets();
        for i in 0..slab_size {
            let undo = PacketSlabUndo {
                position: i,
                old_packet: packets[i],
            };
            packets[i] = LZMAPacket::short_rep_packet();
            stack.insert(undo);
        }
    }
    for p in slab.packets().iter() {
        assert_eq!(p.packet_type, LZMAPacketType::ShortRep);
    }
}

fn main() {}
