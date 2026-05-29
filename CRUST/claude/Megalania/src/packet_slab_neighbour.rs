use crate::lzma_packet::{LZMAPacket, LZMAPacketType};
use crate::lzma_packet_encoder::lzma_encode_packet;
use crate::lzma_state::LZMAState;
use crate::packet_slab::PacketSlab;
use crate::packet_slab_undo_stack::{PacketSlabUndo, PacketSlabUndoStack};
use crate::perplexity_encoder::PerplexityEncoder;
use crate::top_k_packet_finder::TopKPacketFinder;
use rand::Rng;

pub struct PacketSlabNeighbour<'a> {
    pub init_state: LZMAState<'a>,
    pub slab: Box<PacketSlab>,
    pub perplexity: u64,
    pub undo_stack: PacketSlabUndoStack,
}

impl<'a> PacketSlabNeighbour<'a> {
    pub fn new(slab: PacketSlab, init_state: LZMAState<'a>) -> Self {
        Self {
            init_state,
            slab: Box::new(slab),
            perplexity: 0,
            undo_stack: PacketSlabUndoStack::new(),
        }
    }

    pub fn free(self) {
        // PacketSlabUndoStack and PacketSlab are dropped automatically.
    }

    pub fn generate(&mut self, packet_finder: &mut TopKPacketFinder) -> bool {
        let packet_count = self.slab.count();
        if packet_count == 0 {
            return false;
        }
        let mut rng = rand::rng();
        let mutation_target = rng.random_range(0..packet_count);

        let mut lzma_state = self.init_state.clone();
        let mut enc = PerplexityEncoder::new();

        // Encode up to mutation_target.
        encode_to_packet_number(
            &mut lzma_state,
            &mut enc,
            self.slab.packets(),
            mutation_target,
        );

        if !mutate_next_packet(
            &mut self.undo_stack,
            &mut lzma_state,
            packet_finder,
            self.slab.packets(),
        ) {
            self.perplexity = enc.perplexity;
            return false;
        }
        let pkt = self.slab.packets()[lzma_state.position];
        lzma_encode_packet(&mut lzma_state, &mut enc, pkt);

        repair_remaining_packets(
            &mut self.undo_stack,
            &mut lzma_state,
            &mut enc,
            packet_finder,
            self.slab.packets(),
        );
        self.perplexity = enc.perplexity;
        true
    }

    pub fn undo(&mut self) {
        self.undo_stack.apply(&mut self.slab);
    }

    pub fn undo_count(&self) -> usize {
        self.undo_stack.count()
    }
}

fn encode_to_packet_number(
    lzma_state: &mut LZMAState,
    enc: &mut PerplexityEncoder,
    packets: &[LZMAPacket],
    target: usize,
) {
    let mut count = 0;
    while lzma_state.position < lzma_state.data.len() {
        if count == target {
            break;
        }
        count += 1;
        let pkt = packets[lzma_state.position];
        lzma_encode_packet(lzma_state, enc, pkt);
    }
}

fn save_packet_at_position(
    undo_stack: &mut PacketSlabUndoStack,
    packet: LZMAPacket,
    position: usize,
) {
    let undo = PacketSlabUndo {
        position,
        old_packet: packet,
    };
    undo_stack.insert(undo);
}

fn rand_max_dist(count: usize, num: usize) -> usize {
    let mut rng = rand::rng();
    let mut rnd = rng.random_range(0..count);
    let mut num = num;
    while num > 1 {
        num -= 1;
        let r = rng.random_range(0..count);
        if r > rnd {
            rnd = r;
        }
    }
    rnd
}

fn pick_random_next_packet_from_top_k(
    lzma_state: &LZMAState,
    packet_finder: &mut TopKPacketFinder,
    packets: &mut [LZMAPacket],
    best: bool,
) -> bool {
    packet_finder.find(lzma_state, packets);
    let count = packet_finder.count();
    if count == 0 {
        return false;
    }
    let mut rng = rand::rng();
    let mut choice = rand_max_dist(count, 8);
    if rng.random_range(0..8) == 0 || best {
        choice = count - 1;
    }
    while let Some(pkt) = packet_finder.pop() {
        packets[lzma_state.position] = pkt;
        if choice == 0 {
            return true;
        }
        choice -= 1;
    }
    true
}

fn validate_long_rep_packet(lzma_state: &LZMAState, packet: LZMAPacket) -> bool {
    assert!(packet.packet_type == LZMAPacketType::LongRep);
    let dist = lzma_state.dists[packet.dist as usize] as usize;
    let pos = lzma_state.position;
    if pos == 0 || pos < dist + 1 {
        return false;
    }
    let start = pos - dist - 1;
    let len = packet.len as usize;
    if start + len > lzma_state.data.len() || pos + len > lzma_state.data.len() {
        return false;
    }
    lzma_state.data[start..start + len] == lzma_state.data[pos..pos + len]
}

fn repair_remaining_packets(
    undo_stack: &mut PacketSlabUndoStack,
    lzma_state: &mut LZMAState,
    enc: &mut PerplexityEncoder,
    packet_finder: &mut TopKPacketFinder,
    packets: &mut [LZMAPacket],
) {
    let mut count = 0usize;
    while lzma_state.position < lzma_state.data.len() {
        count += 1;
        let pos = lzma_state.position;
        let old_packet = packets[pos];
        let mut packet = old_packet;

        if packet.packet_type == LZMAPacketType::ShortRep
            || packet.packet_type == LZMAPacketType::Literal
        {
            let dist0 = lzma_state.dists[0] as usize;
            if pos > 0
                && pos > dist0
                && lzma_state.data[pos] == lzma_state.data[pos - dist0 - 1]
            {
                if count < 4 {
                    packet = LZMAPacket::short_rep_packet();
                }
            } else {
                packet = LZMAPacket::literal_packet();
            }
        }
        if packet.packet_type == LZMAPacketType::LongRep {
            let mut dist_index = 0u32;
            while !validate_long_rep_packet(lzma_state, packet) && dist_index < 4 {
                packet.dist = dist_index;
                dist_index += 1;
            }
            if !validate_long_rep_packet(lzma_state, packet) {
                let mut rng = rand::rng();
                pick_random_next_packet_from_top_k(
                    lzma_state,
                    packet_finder,
                    packets,
                    rng.random_range(0..4) == 0,
                );
                packet = packets[pos];
            }
        }

        if !LZMAPacket::cmp(&old_packet, &packet) {
            save_packet_at_position(undo_stack, old_packet, pos);
        }
        packets[pos] = packet;

        lzma_encode_packet(lzma_state, enc, packet);
    }
}

fn mutate_next_packet(
    undo_stack: &mut PacketSlabUndoStack,
    lzma_state: &LZMAState,
    packet_finder: &mut TopKPacketFinder,
    packets: &mut [LZMAPacket],
) -> bool {
    let mut rng = rand::rng();
    let pos = lzma_state.position;
    let data_size = lzma_state.data.len();

    if pos + 1 < data_size && rng.random_range(0..2) == 0 {
        let first_packet = packets[pos];
        let second_packet = packets[pos + 1];
        if (first_packet.packet_type == LZMAPacketType::LongRep
            || first_packet.packet_type == LZMAPacketType::Match)
            && first_packet.len > 2
        {
            save_packet_at_position(undo_stack, first_packet, pos);
            save_packet_at_position(undo_stack, second_packet, pos + 1);
            let mut new_second = first_packet;
            new_second.len -= 1;
            packets[pos + 1] = new_second;
            packets[pos] = LZMAPacket::literal_packet();
            return true;
        } else if first_packet.packet_type == LZMAPacketType::Literal
            || first_packet.packet_type == LZMAPacketType::ShortRep
        {
            if second_packet.packet_type == LZMAPacketType::Match
                || second_packet.packet_type == LZMAPacketType::LongRep
            {
                let rep_start: i64 = match second_packet.packet_type {
                    LZMAPacketType::LongRep => {
                        pos as i64 - lzma_state.dists[second_packet.dist as usize] as i64
                    }
                    _ => pos as i64 - second_packet.dist as i64,
                };
                if second_packet.len < 273
                    && rep_start > 0
                    && (rep_start as usize) - 1 < data_size
                    && lzma_state.data[pos] == lzma_state.data[(rep_start as usize) - 1]
                {
                    save_packet_at_position(undo_stack, first_packet, pos);
                    let mut new_first = second_packet;
                    new_first.len += 1;
                    packets[pos] = new_first;
                    return true;
                }
            }
        }
    }

    save_packet_at_position(undo_stack, packets[pos], pos);
    pick_random_next_packet_from_top_k(lzma_state, packet_finder, packets, false)
}
