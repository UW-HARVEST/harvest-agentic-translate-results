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
    let mut n = num;
    while n > 1 {
        n -= 1;
        let r = rng.gen_range(0..count);
        if r > rnd {
            rnd = r;
        }
    }
    rnd
}

fn validate_long_rep_packet(lzma_state: &LZMAState, packet: LZMAPacket) -> bool {
    assert_eq!(packet.packet_type, LZMAPacketType::LongRep);
    let dist = lzma_state.dists[packet.dist as usize] as usize;
    if lzma_state.position < dist + 1 {
        return false;
    }
    let rep_start_idx = lzma_state.position - dist - 1;
    let len = packet.len as usize;
    if rep_start_idx + len > lzma_state.data.len() || lzma_state.position + len > lzma_state.data.len() {
        return false;
    }
    let rep_slice = &lzma_state.data[rep_start_idx..rep_start_idx + len];
    let cur_slice = &lzma_state.data[lzma_state.position..lzma_state.position + len];
    rep_slice == cur_slice
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
        // Resources freed automatically via Drop.
    }
    pub fn generate(&mut self, packet_finder: &mut TopKPacketFinder) -> bool {
        let init_state_clone = self.init_state.clone();
        let mut lzma_state = init_state_clone;

        let packet_count = self.slab.count();
        if packet_count == 0 {
            return false;
        }

        let mut rng = rand::thread_rng();
        let mutation_target = rng.gen_range(0..packet_count);

        // Encode up to mutation_target packets.
        let mut perplexity_local: u64 = 0;
        {
            let mut enc = PerplexityEncoder {
                perplexity: &mut perplexity_local,
            };
            let packets = self.slab.packets().to_vec();
            let mut count = 0;
            while lzma_state.position < lzma_state.data.len() {
                if count == mutation_target {
                    break;
                }
                count += 1;
                let pkt = packets[lzma_state.position];
                lzma_encode_packet(&mut lzma_state, &mut enc, pkt);
            }
        }

        // Mutate next packet.
        if !self.mutate_next_packet(&lzma_state, packet_finder) {
            return false;
        }

        // Now encode the next packet (which may have been mutated).
        let next_pkt = self.slab.packets()[lzma_state.position];
        {
            let mut enc = PerplexityEncoder {
                perplexity: &mut perplexity_local,
            };
            lzma_encode_packet(&mut lzma_state, &mut enc, next_pkt);
        }

        // Repair remaining packets.
        self.repair_remaining_packets(&mut lzma_state, packet_finder, &mut perplexity_local);

        self.perplexity = perplexity_local;
        true
    }

    fn mutate_next_packet(
        &mut self,
        lzma_state: &LZMAState,
        packet_finder: &mut TopKPacketFinder,
    ) -> bool {
        let mut rng = rand::thread_rng();
        let pos = lzma_state.position;
        let data_len = lzma_state.data.len();

        // Try to randomly grow/shrink the next match packet.
        if pos + 1 < data_len && rng.gen_range(0..2) == 0 {
            let first_packet = self.slab.packets()[pos];
            let second_packet = self.slab.packets()[pos + 1];
            if (first_packet.packet_type == LZMAPacketType::LongRep
                || first_packet.packet_type == LZMAPacketType::Match)
                && first_packet.len > 2
            {
                self.save_packet_at_position(first_packet, pos);
                self.save_packet_at_position(second_packet, pos + 1);
                {
                    let packets = self.slab.packets();
                    packets[pos + 1] = first_packet;
                    packets[pos + 1].len -= 1;
                    packets[pos] = LZMAPacket::literal_packet();
                }
                return true;
            } else if first_packet.packet_type == LZMAPacketType::Literal
                || first_packet.packet_type == LZMAPacketType::ShortRep
            {
                if second_packet.packet_type == LZMAPacketType::Match
                    || second_packet.packet_type == LZMAPacketType::LongRep
                {
                    let mut rep_start = pos as isize - second_packet.dist as isize;
                    if second_packet.packet_type == LZMAPacketType::LongRep {
                        rep_start =
                            pos as isize - lzma_state.dists[second_packet.dist as usize] as isize;
                    }
                    if second_packet.len < 273
                        && rep_start > 0
                        && (rep_start as usize) - 1 < data_len
                        && lzma_state.data[pos] == lzma_state.data[(rep_start as usize) - 1]
                    {
                        self.save_packet_at_position(first_packet, pos);
                        let packets = self.slab.packets();
                        packets[pos] = second_packet;
                        packets[pos].len += 1;
                        return true;
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
        let pos = lzma_state.position;
        // Build a copy of next_packets for the finder to use.
        let mut next_packets_copy = self.slab.packets().to_vec();
        packet_finder.find(lzma_state, &mut next_packets_copy);
        let count = packet_finder.count();
        if count == 0 {
            return false;
        }

        let mut rng = rand::thread_rng();
        let mut choice = rand_max_dist(count, 8);
        if rng.gen_range(0..8) == 0 || best {
            choice = count - 1;
        }
        // Pop until choice == 0
        let packets = self.slab.packets();
        while let Some(packet) = packet_finder.pop() {
            if choice == 0 {
                packets[pos] = packet;
                return true;
            }
            choice -= 1;
        }
        true
    }

    fn repair_remaining_packets(
        &mut self,
        lzma_state: &mut LZMAState,
        packet_finder: &mut TopKPacketFinder,
        perplexity_local: &mut u64,
    ) {
        let mut count: usize = 0;
        let mut rng_seed_count = 0u32;
        let _ = rng_seed_count;
        let mut rng = rand::thread_rng();
        while lzma_state.position < lzma_state.data.len() {
            count += 1;
            let pos = lzma_state.position;
            let old_packet = self.slab.packets()[pos];
            let mut new_packet = old_packet;

            if new_packet.packet_type == LZMAPacketType::ShortRep
                || new_packet.packet_type == LZMAPacketType::Literal
            {
                let dist0 = lzma_state.dists[0] as usize;
                if pos >= dist0 + 1 && lzma_state.data[pos] == lzma_state.data[pos - dist0 - 1] {
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
                    let pick_best = rng.gen_range(0..4u32) == 0;
                    self.slab.packets()[pos] = new_packet;
                    self.pick_random_next_packet_from_top_k(lzma_state, packet_finder, pick_best);
                    new_packet = self.slab.packets()[pos];
                }
            }

            if !LZMAPacket::cmp(&old_packet, &new_packet) {
                self.save_packet_at_position(old_packet, pos);
                self.slab.packets()[pos] = new_packet;
            }

            let pkt = self.slab.packets()[pos];
            let mut enc = PerplexityEncoder {
                perplexity: perplexity_local,
            };
            lzma_encode_packet(lzma_state, &mut enc, pkt);
        }
    }

    pub fn undo(&mut self) {
        self.undo_stack.apply(&mut self.slab);
    }
    pub fn undo_count(&self) -> usize {
        self.undo_stack.count()
    }
}
