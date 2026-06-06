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

fn rand_max_dist(count: usize, num: usize) -> usize {
    let mut rng = rand::thread_rng();
    let mut rnd = rng.gen_range(0..count);
    let mut num = num;
    while num > 1 {
        num -= 1;
        let r = rng.gen_range(0..count);
        if r > rnd {
            rnd = r;
        }
    }
    rnd
}

fn validate_long_rep_packet(lzma_state: &LZMAState, packet: LZMAPacket) -> bool {
    assert!(packet.packet_type == LZMAPacketType::LongRep);
    let dist = lzma_state.dists[packet.dist as usize] as usize;
    let pos = lzma_state.position;
    if pos < dist + 1 {
        return false;
    }
    let rep_start = pos - dist - 1;
    let len = packet.len as usize;
    if rep_start + len > lzma_state.data.len() || pos + len > lzma_state.data.len() {
        return false;
    }
    lzma_state.data[rep_start..rep_start + len] == lzma_state.data[pos..pos + len]
}

impl<'a> PacketSlabNeighbour<'a> {
    pub fn new(slab: PacketSlab, init_state: LZMAState<'a>) -> Self {
        PacketSlabNeighbour {
            init_state,
            slab: Box::new(slab),
            perplexity: 0,
            undo_stack: PacketSlabUndoStack::new(),
        }
    }
    pub fn free(self) {
        // Resources are dropped automatically.
    }
    pub fn generate(&mut self, packet_finder: &mut TopKPacketFinder) -> bool {
        let packet_count = self.slab.count();
        if packet_count == 0 {
            return false;
        }
        let mut rng = rand::thread_rng();
        let mutation_target = rng.gen_range(0..packet_count);

        // Begin perplexity counting.
        self.perplexity = 0;

        // Encode up to packet number `mutation_target`.
        let mut lzma_state = self.init_state.clone();
        encode_to_packet_number(
            &mut lzma_state,
            &mut self.perplexity,
            self.slab.packets(),
            mutation_target,
        );

        // Mutate the next packet.
        let mutated = mutate_next_packet(
            &mut self.undo_stack,
            &lzma_state,
            packet_finder,
            self.slab.packets(),
        );
        if !mutated {
            return false;
        }

        // Encode mutated packet.
        {
            let mut enc = PerplexityEncoder::new(&mut self.perplexity);
            let packet = self.slab.packets()[lzma_state.position];
            lzma_encode_packet(&mut lzma_state, &mut enc, packet);
        }

        // Repair remaining packets.
        repair_remaining_packets(
            &mut self.undo_stack,
            &mut lzma_state,
            &mut self.perplexity,
            packet_finder,
            self.slab.packets(),
        );
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
    perplexity: &mut u64,
    packets: &[LZMAPacket],
    target: usize,
) {
    let mut count = 0;
    let mut enc = PerplexityEncoder::new(perplexity);
    while lzma_state.position < lzma_state.data.len() {
        if count == target {
            break;
        }
        count += 1;
        let p = packets[lzma_state.position];
        lzma_encode_packet(lzma_state, &mut enc, p);
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
    let mut rng = rand::thread_rng();
    let mut choice = rand_max_dist(count, 8);
    if rng.gen_range(0..8) == 0 || best {
        choice = count - 1;
    }
    while let Some(packet) = packet_finder.pop() {
        if choice == 0 {
            packets[lzma_state.position] = packet;
            return true;
        }
        choice -= 1;
    }
    true
}

fn mutate_next_packet(
    undo_stack: &mut PacketSlabUndoStack,
    lzma_state: &LZMAState,
    packet_finder: &mut TopKPacketFinder,
    packets: &mut [LZMAPacket],
) -> bool {
    let pos = lzma_state.position;
    let mut rng = rand::thread_rng();

    if pos + 1 < lzma_state.data.len() && rng.gen_range(0..2) == 0 {
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
                let rep_start = if second_packet.packet_type == LZMAPacketType::LongRep {
                    let dist = lzma_state.dists[second_packet.dist as usize] as usize;
                    if pos < dist {
                        return false;
                    }
                    pos - dist
                } else {
                    if pos < second_packet.dist as usize {
                        return false;
                    }
                    pos - second_packet.dist as usize
                };
                if second_packet.len < 273
                    && rep_start > 0
                    && lzma_state.data[pos] == lzma_state.data[rep_start - 1]
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
    if pick_random_next_packet_from_top_k(lzma_state, packet_finder, packets, false) {
        return true;
    }
    false
}

fn repair_remaining_packets(
    undo_stack: &mut PacketSlabUndoStack,
    lzma_state: &mut LZMAState,
    perplexity: &mut u64,
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
            if pos > dist0
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
            let mut dist_index: u32 = 0;
            while !validate_long_rep_packet(lzma_state, packet) && dist_index < 4 {
                packet.dist = dist_index;
                dist_index += 1;
            }
            if !validate_long_rep_packet(lzma_state, packet) {
                packets[pos] = packet;
                let mut rng = rand::thread_rng();
                let best = rng.gen_range(0..4) == 0;
                pick_random_next_packet_from_top_k(lzma_state, packet_finder, packets, best);
                packet = packets[pos];
            }
        }

        if !LZMAPacket::cmp(&old_packet, &packet) {
            save_packet_at_position(undo_stack, old_packet, pos);
        }
        packets[pos] = packet;

        let mut enc = PerplexityEncoder::new(perplexity);
        lzma_encode_packet(lzma_state, &mut enc, packet);
    }
}
