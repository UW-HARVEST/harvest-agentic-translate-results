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
    let mut rng = rand::rng();
    let mut rnd: usize = rng.random_range(0..count);
    let mut n = num;
    while n > 1 {
        let r = rng.random_range(0..count);
        if r > rnd {
            rnd = r;
        }
        n -= 1;
    }
    rnd
}

fn validate_long_rep_packet(lzma_state: &LZMAState, packet: LZMAPacket) -> bool {
    assert!(packet.packet_type == LZMAPacketType::LongRep);
    let dist_index = packet.dist as usize;
    let position = lzma_state.position;
    let rep_start = position - lzma_state.dists[dist_index] as usize - 1;
    let len = packet.len as usize;
    if rep_start + len > lzma_state.data.len() {
        return false;
    }
    if position + len > lzma_state.data.len() {
        return false;
    }
    lzma_state.data[rep_start..rep_start + len] == lzma_state.data[position..position + len]
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
        // Drops the slab and the undo stack.
        drop(self);
    }
    pub fn generate(&mut self, packet_finder: &mut TopKPacketFinder) -> bool {
        // Clone the initial state (LZMAState borrows the data slice).
        let mut lzma_state = self.init_state.clone();
        let mut perplexity_value: u64 = 0;
        let packets = self.slab.packets();
        let packets_len = packets.len();

        // Compute packet count by walking the slab.
        let packet_count = {
            let mut position = 0usize;
            let mut count = 0usize;
            while position < packets_len {
                count += 1;
                position += packets[position].len as usize;
            }
            count
        };
        if packet_count == 0 {
            return false;
        }

        let mutation_target: usize = {
            let mut rng = rand::rng();
            rng.random_range(0..packet_count)
        };

        // Encode up to the target packet number.
        {
            let mut enc = PerplexityEncoder::new(&mut perplexity_value);
            let mut count = 0usize;
            while lzma_state.position < lzma_state.data.len() {
                if count == mutation_target {
                    break;
                }
                count += 1;
                let pkt = packets[lzma_state.position];
                lzma_encode_packet(&mut lzma_state, &mut enc, pkt);
            }
        }

        // Now mutate the next packet.
        if !self.mutate_next_packet(&lzma_state, packet_finder) {
            self.perplexity = perplexity_value;
            return false;
        }

        // Encode the (now mutated) packet at the current position.
        {
            let pkt = self.slab.packets()[lzma_state.position];
            let mut enc = PerplexityEncoder::new(&mut perplexity_value);
            lzma_encode_packet(&mut lzma_state, &mut enc, pkt);
        }

        // Repair remaining packets and encode them.
        self.repair_remaining_packets(&mut lzma_state, &mut perplexity_value, packet_finder);

        self.perplexity = perplexity_value;
        true
    }
    pub fn undo(&mut self) {
        self.undo_stack.apply(&mut self.slab);
    }
    pub fn undo_count(&self) -> usize {
        self.undo_stack.count()
    }

    fn save_packet_at_position(&mut self, packet: LZMAPacket, position: usize) {
        self.undo_stack.insert(PacketSlabUndo {
            position,
            old_packet: packet,
        });
    }

    fn pick_random_next_packet_from_top_k(
        &mut self,
        lzma_state: &LZMAState,
        packet_finder: &mut TopKPacketFinder,
        best: bool,
    ) -> bool {
        let position = lzma_state.position;
        let packets = self.slab.packets();
        packet_finder.find(lzma_state, packets);
        let count = packet_finder.count();
        if count == 0 {
            return false;
        }
        let mut choice: usize = rand_max_dist(count, 8);
        let mut rng = rand::rng();
        if rng.random_range(0..8) == 0 || best {
            choice = count - 1;
        }
        loop {
            let popped = packet_finder.pop();
            match popped {
                None => return true,
                Some(p) => {
                    if choice == 0 {
                        let packets = self.slab.packets();
                        packets[position] = p;
                        return true;
                    }
                    choice -= 1;
                }
            }
        }
    }

    fn mutate_next_packet(
        &mut self,
        lzma_state: &LZMAState,
        packet_finder: &mut TopKPacketFinder,
    ) -> bool {
        let position = lzma_state.position;
        let data_size = lzma_state.data.len();
        let mut rng = rand::rng();

        if position + 1 < data_size && rng.random_range(0..2) == 0 {
            let first_packet = self.slab.packets()[position];
            let second_packet = self.slab.packets()[position + 1];
            // Try shrinking a long match into a literal followed by a shorter match.
            if (first_packet.packet_type == LZMAPacketType::LongRep
                || first_packet.packet_type == LZMAPacketType::Match)
                && first_packet.len > 2
            {
                self.save_packet_at_position(first_packet, position);
                self.save_packet_at_position(second_packet, position + 1);
                let mut new_second = first_packet;
                new_second.len -= 1;
                let new_first = LZMAPacket::literal_packet();
                let packets = self.slab.packets();
                packets[position + 1] = new_second;
                packets[position] = new_first;
                return true;
            } else if first_packet.packet_type == LZMAPacketType::Literal
                || first_packet.packet_type == LZMAPacketType::ShortRep
            {
                if second_packet.packet_type == LZMAPacketType::Match
                    || second_packet.packet_type == LZMAPacketType::LongRep
                {
                    let mut rep_start: i64 = position as i64 - second_packet.dist as i64;
                    if second_packet.packet_type == LZMAPacketType::LongRep {
                        rep_start = position as i64
                            - lzma_state.dists[second_packet.dist as usize] as i64;
                    }
                    if second_packet.len < 273
                        && rep_start > 0
                        && lzma_state.data[position]
                            == lzma_state.data[(rep_start - 1) as usize]
                    {
                        self.save_packet_at_position(first_packet, position);
                        let mut new_first = second_packet;
                        new_first.len += 1;
                        let packets = self.slab.packets();
                        packets[position] = new_first;
                        return true;
                    }
                }
            }
        }

        let saved = self.slab.packets()[position];
        self.save_packet_at_position(saved, position);
        if self.pick_random_next_packet_from_top_k(lzma_state, packet_finder, false) {
            return true;
        }
        false
    }

    fn repair_remaining_packets(
        &mut self,
        lzma_state: &mut LZMAState,
        perplexity_value: &mut u64,
        packet_finder: &mut TopKPacketFinder,
    ) {
        let mut count = 0usize;
        while lzma_state.position < lzma_state.data.len() {
            count += 1;
            let position = lzma_state.position;
            let old_packet = self.slab.packets()[position];
            let mut new_packet = old_packet;

            if new_packet.packet_type == LZMAPacketType::ShortRep
                || new_packet.packet_type == LZMAPacketType::Literal
            {
                let dist0 = lzma_state.dists[0] as usize;
                if position >= dist0 + 1
                    && lzma_state.data[position] == lzma_state.data[position - dist0 - 1]
                {
                    if count < 4 {
                        new_packet = LZMAPacket::short_rep_packet();
                    }
                } else {
                    new_packet = LZMAPacket::literal_packet();
                }
            }

            if new_packet.packet_type == LZMAPacketType::LongRep {
                let mut dist_index: u32 = 0;
                while !validate_long_rep_packet(lzma_state, new_packet) && dist_index < 4 {
                    new_packet.dist = dist_index;
                    dist_index += 1;
                }
                if !validate_long_rep_packet(lzma_state, new_packet) {
                    // We don't have to worry about not having a second packet here
                    // because there will always be the literal packet.
                    let mut rng = rand::rng();
                    let best = rng.random_range(0..4) == 0;
                    self.pick_random_next_packet_from_top_k(lzma_state, packet_finder, best);
                    new_packet = self.slab.packets()[position];
                }
            }

            // Write back the chosen packet (may have been written by pick_random already).
            {
                let packets = self.slab.packets();
                packets[position] = new_packet;
            }

            if !LZMAPacket::cmp(&old_packet, &new_packet) {
                self.save_packet_at_position(old_packet, position);
            }

            // Encode the packet using a perplexity encoder.
            {
                let pkt = self.slab.packets()[position];
                let mut enc = PerplexityEncoder::new(perplexity_value);
                lzma_encode_packet(lzma_state, &mut enc, pkt);
            }
        }
    }
}
