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
        // Dropping self is sufficient; the undo stack and slab clean themselves up.
    }
    pub fn generate(&mut self, packet_finder: &mut TopKPacketFinder) -> bool {
        let mut lzma_state = self.init_state.clone();
        let mut perplexity: u64 = 0;

        let packet_count = self.slab.count();
        if packet_count == 0 {
            return false;
        }
        let mutation_target = rand::rng().random_range(0..packet_count);

        // Encode packets up to (but not including) the target packet number.
        {
            let mut enc = PerplexityEncoder::new(&mut perplexity);
            let mut count: usize = 0;
            while lzma_state.position < lzma_state.data.len() && count < mutation_target {
                count += 1;
                let packet = self.slab.packets()[lzma_state.position];
                lzma_encode_packet(&mut lzma_state, &mut enc, packet);
            }
        }

        // Mutate the next packet in some way.
        if !self.mutate_next_packet(&lzma_state, packet_finder) {
            return false;
        }

        // Encode the (newly mutated) packet at the current position.
        {
            let mut enc = PerplexityEncoder::new(&mut perplexity);
            let packet = self.slab.packets()[lzma_state.position];
            lzma_encode_packet(&mut lzma_state, &mut enc, packet);

            // Repair and encode any remaining packets.
            self.repair_remaining_packets(&mut lzma_state, &mut enc, packet_finder);
        }

        self.perplexity = perplexity;
        true
    }
    pub fn undo(&mut self) {
        self.undo_stack.apply(&mut self.slab);
    }
    pub fn undo_count(&self) -> usize {
        self.undo_stack.count()
    }

    fn save_packet_at_position(&mut self, packet: LZMAPacket, position: usize) {
        let undo = PacketSlabUndo {
            position,
            old_packet: packet,
        };
        self.undo_stack.insert(undo);
    }

    fn pick_random_next_packet_from_top_k(
        &mut self,
        lzma_state: &LZMAState,
        packet_finder: &mut TopKPacketFinder,
        best: bool,
    ) -> bool {
        let packets = self.slab.packets();
        packet_finder.find(lzma_state, packets);
        let count = packet_finder.count();
        if count == 0 {
            return false;
        }
        let mut rng = rand::rng();
        let mut choice = {
            let mut rnd = rng.random_range(0..count);
            for _ in 1..8 {
                let v = rng.random_range(0..count);
                if v > rnd {
                    rnd = v;
                }
            }
            rnd
        };
        if rng.random_range(0..8) == 0 || best {
            choice = count - 1;
        }
        while let Some(p) = packet_finder.pop() {
            if choice == 0 {
                packets[lzma_state.position] = p;
                return true;
            }
            choice -= 1;
        }
        true
    }

    fn validate_long_rep_packet(lzma_state: &LZMAState, packet: LZMAPacket) -> bool {
        assert!(packet.packet_type == LZMAPacketType::LongRep);
        let dist_index = packet.dist as usize;
        if dist_index >= 4 {
            return false;
        }
        let dist = lzma_state.dists[dist_index] as usize;
        if lzma_state.position == 0 || lzma_state.position <= dist {
            return false;
        }
        let rep_start = lzma_state.position - dist - 1;
        let len = packet.len as usize;
        if rep_start + len > lzma_state.data.len() || lzma_state.position + len > lzma_state.data.len() {
            return false;
        }
        lzma_state.data[rep_start..rep_start + len]
            == lzma_state.data[lzma_state.position..lzma_state.position + len]
    }

    fn mutate_next_packet(
        &mut self,
        lzma_state: &LZMAState,
        packet_finder: &mut TopKPacketFinder,
    ) -> bool {
        let pos = lzma_state.position;
        if pos >= lzma_state.data.len() {
            return false;
        }
        let mut rng = rand::rng();

        let first_packet = self.slab.packets()[pos];
        if pos + 1 < lzma_state.data.len() && rng.random_range(0..2) == 0 {
            let second_packet = self.slab.packets()[pos + 1];
            if (first_packet.packet_type == LZMAPacketType::LongRep
                || first_packet.packet_type == LZMAPacketType::Match)
                && first_packet.len > 2
            {
                self.save_packet_at_position(first_packet, pos);
                self.save_packet_at_position(second_packet, pos + 1);
                let mut new_second = first_packet;
                new_second.len -= 1;
                self.slab.packets()[pos + 1] = new_second;
                self.slab.packets()[pos] = LZMAPacket::literal_packet();
                return true;
            } else if first_packet.packet_type == LZMAPacketType::Literal
                || first_packet.packet_type == LZMAPacketType::ShortRep
            {
                if second_packet.packet_type == LZMAPacketType::Match
                    || second_packet.packet_type == LZMAPacketType::LongRep
                {
                    let rep_start = if second_packet.packet_type == LZMAPacketType::LongRep {
                        let dist_index = second_packet.dist as usize;
                        if dist_index < 4 {
                            pos.checked_sub(lzma_state.dists[dist_index] as usize)
                        } else {
                            None
                        }
                    } else {
                        pos.checked_sub(second_packet.dist as usize)
                    };
                    if let Some(rep_start) = rep_start {
                        if second_packet.len < 273
                            && rep_start > 0
                            && lzma_state.data[pos] == lzma_state.data[rep_start - 1]
                        {
                            self.save_packet_at_position(first_packet, pos);
                            let mut new_first = second_packet;
                            new_first.len += 1;
                            self.slab.packets()[pos] = new_first;
                            return true;
                        }
                    }
                }
            }
        }
        let cur_packet = self.slab.packets()[pos];
        self.save_packet_at_position(cur_packet, pos);
        if self.pick_random_next_packet_from_top_k(lzma_state, packet_finder, false) {
            return true;
        }
        false
    }

    fn repair_remaining_packets(
        &mut self,
        lzma_state: &mut LZMAState,
        enc: &mut PerplexityEncoder,
        packet_finder: &mut TopKPacketFinder,
    ) {
        let mut count: usize = 0;
        while lzma_state.position < lzma_state.data.len() {
            count += 1;
            let pos = lzma_state.position;
            let old_packet = self.slab.packets()[pos];
            let mut packet = old_packet;

            if packet.packet_type == LZMAPacketType::ShortRep
                || packet.packet_type == LZMAPacketType::Literal
            {
                let dist0 = lzma_state.dists[0] as usize;
                let can_short_rep = pos > dist0
                    && lzma_state.data[pos] == lzma_state.data[pos - dist0 - 1];
                if can_short_rep {
                    if count < 4 {
                        packet = LZMAPacket::short_rep_packet();
                    }
                } else {
                    packet = LZMAPacket::literal_packet();
                }
            }
            if packet.packet_type == LZMAPacketType::LongRep {
                let mut dist_index: u32 = 0;
                while !Self::validate_long_rep_packet(lzma_state, packet) && dist_index < 4 {
                    packet.dist = dist_index;
                    dist_index += 1;
                }
                if !Self::validate_long_rep_packet(lzma_state, packet) {
                    self.slab.packets()[pos] = packet;
                    let _ = self.pick_random_next_packet_from_top_k(
                        lzma_state,
                        packet_finder,
                        rand::rng().random_range(0..4) == 0,
                    );
                    packet = self.slab.packets()[pos];
                }
            }

            if !LZMAPacket::cmp(&old_packet, &packet) {
                self.save_packet_at_position(old_packet, pos);
                self.slab.packets()[pos] = packet;
            }

            lzma_encode_packet(lzma_state, enc, packet);
        }
    }
}
