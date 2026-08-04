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

fn rand_max_dist(count: usize, mut num: usize) -> usize {
    let mut rng = rand::rng();
    let mut rnd = rng.random_range(0..count);
    num -= 1;
    while num != 0 {
        rnd = rnd.max(rng.random_range(0..count));
        num -= 1;
    }
    rnd
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
        // Drop happens automatically
    }

    fn save_packet_at_position(&mut self, packet: LZMAPacket, position: usize) {
        self.undo_stack.insert(PacketSlabUndo {
            position,
            old_packet: packet,
        });
    }

    fn encode_to_packet_number(
        lzma_state: &mut LZMAState,
        enc: &mut dyn crate::encoder_interface::EncoderInterface,
        packets: &[LZMAPacket],
        target: usize,
    ) {
        let mut count = 0;
        while lzma_state.position < lzma_state.data.len() {
            if count == target {
                break;
            }
            count += 1;
            lzma_encode_packet(lzma_state, enc, packets[lzma_state.position]);
        }
    }

    fn pick_random_next_packet_from_top_k(
        lzma_state: &LZMAState,
        packet_finder: &mut TopKPacketFinder,
        packets: &mut [LZMAPacket],
        best: bool,
    ) -> bool {
        let mut rng = rand::rng();
        packet_finder.find(lzma_state, packets);
        let count = packet_finder.count();
        if count == 0 {
            return false;
        }
        let mut choice = rand_max_dist(count, 8);
        if rng.random_range(0..8u32) == 0 || best {
            choice = count - 1;
        }
        let pos = lzma_state.position;
        let mut remaining = choice;
        while let Some(p) = packet_finder.pop() {
            if remaining == 0 {
                packets[pos] = p;
                return true;
            }
            remaining -= 1;
        }
        false
    }

    fn validate_long_rep_packet(lzma_state: &LZMAState, packet: LZMAPacket) -> bool {
        let rep_start = lzma_state.position.wrapping_sub(lzma_state.dists[packet.dist as usize] as usize + 1);
        let current = lzma_state.position;
        let len = packet.len as usize;
        if rep_start + len > lzma_state.data.len() || current + len > lzma_state.data.len() {
            return false;
        }
        lzma_state.data[rep_start..rep_start + len] == lzma_state.data[current..current + len]
    }

    fn repair_remaining_packets(
        &mut self,
        lzma_state: &mut LZMAState,
        enc: &mut dyn crate::encoder_interface::EncoderInterface,
        packet_finder: &mut TopKPacketFinder,
    ) {
        let mut rng = rand::rng();
        let mut count: usize = 0;
        while lzma_state.position < lzma_state.data.len() {
            count += 1;
            let pos = lzma_state.position;
            let old_packet = self.slab.packets()[pos];

            let mut packet = old_packet;

            if packet.packet_type == LZMAPacketType::ShortRep
                || packet.packet_type == LZMAPacketType::Literal
            {
                let rep0_pos = pos.wrapping_sub(lzma_state.dists[0] as usize + 1);
                if rep0_pos < lzma_state.data.len()
                    && lzma_state.data[pos] == lzma_state.data[rep0_pos]
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
                let mut temp = packet;
                while !Self::validate_long_rep_packet(lzma_state, temp) && dist_index < 4 {
                    temp.dist = dist_index;
                    dist_index += 1;
                }
                packet = temp;
                if !Self::validate_long_rep_packet(lzma_state, packet) {
                    let best = rng.random_range(0..4u32) == 0;
                    Self::pick_random_next_packet_from_top_k(
                        lzma_state,
                        packet_finder,
                        self.slab.packets(),
                        best,
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

    fn mutate_next_packet(
        &mut self,
        lzma_state: &LZMAState,
        packet_finder: &mut TopKPacketFinder,
    ) -> bool {
        let mut rng = rand::rng();
        let pos = lzma_state.position;

        if pos + 1 < lzma_state.data.len() && rng.random_range(0..2u32) == 0 {
            let first_packet = self.slab.packets()[pos];
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
                        pos.wrapping_sub(lzma_state.dists[second_packet.dist as usize] as usize)
                    } else {
                        pos.wrapping_sub(second_packet.dist as usize)
                    };
                    if second_packet.len < 273
                        && rep_start > 0
                        && rep_start <= lzma_state.data.len()
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

        let pkt = self.slab.packets()[pos];
        self.save_packet_at_position(pkt, pos);
        Self::pick_random_next_packet_from_top_k(
            lzma_state,
            packet_finder,
            self.slab.packets(),
            false,
        )
    }

    pub fn generate(&mut self, packet_finder: &mut TopKPacketFinder) -> bool {
        let mut rng = rand::rng();
        let mut lzma_state = self.init_state.clone();
        self.perplexity = 0;
        let mut perplexity_val: u64 = 0;

        let packet_count = self.slab.count();
        let mutation_target = rng.random_range(0..packet_count);

        // Encode to mutation target
        {
            let mut enc = PerplexityEncoder::new(&mut perplexity_val);
            let packets: *const [LZMAPacket] = self.slab.packets();
            let packets_ref = unsafe { &*packets };
            Self::encode_to_packet_number(&mut lzma_state, &mut enc, packets_ref, mutation_target);
        }

        if !self.mutate_next_packet(&lzma_state, packet_finder) {
            self.perplexity = perplexity_val;
            return false;
        }

        // Encode the mutated packet
        {
            let mut enc = PerplexityEncoder::new(&mut perplexity_val);
            let packet = self.slab.packets()[lzma_state.position];
            lzma_encode_packet(&mut lzma_state, &mut enc, packet);
        }

        // Repair remaining
        {
            let mut enc = PerplexityEncoder::new(&mut perplexity_val);
            self.repair_remaining_packets(&mut lzma_state, &mut enc, packet_finder);
        }

        self.perplexity = perplexity_val;
        true
    }

    pub fn undo(&mut self) {
        self.undo_stack.apply(&mut self.slab);
    }

    pub fn undo_count(&self) -> usize {
        self.undo_stack.count()
    }
}
