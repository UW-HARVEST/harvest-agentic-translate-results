use crate::lzma_packet::LZMAPacket;
use crate::lzma_packet::LZMAPacketType;
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
    pub fn new(slab: PacketSlab, init_state: LZMAState) -> Self {
        let init_state: LZMAState<'a> = unsafe { std::mem::transmute(init_state) };
        PacketSlabNeighbour {
            init_state,
            slab: Box::new(slab),
            perplexity: 0,
            undo_stack: PacketSlabUndoStack::new(),
        }
    }

    pub fn free(self) {}

    pub fn generate(&mut self, packet_finder: &mut TopKPacketFinder) -> bool {
        let mut rng = rand::rng();
        let mut lzma_state = self.init_state.clone();
        let mut perplexity: u64 = 0;

        let packet_count = self.slab.count();
        let mutation_target = rng.random_range(0..packet_count);

        // encode_to_packet_number
        {
            let mut enc = PerplexityEncoder { perplexity: &mut perplexity };
            let mut count = 0;
            while lzma_state.position < lzma_state.data.len() {
                if count == mutation_target { break; }
                count += 1;
                let pkt = self.slab.packets()[lzma_state.position];
                lzma_encode_packet(&mut lzma_state, &mut enc, pkt);
            }
        }

        // mutate_next_packet
        if !self.mutate_next_packet(&lzma_state, packet_finder, &mut rng) {
            return false;
        }

        {
            let mut enc = PerplexityEncoder { perplexity: &mut perplexity };
            let pkt = self.slab.packets()[lzma_state.position];
            lzma_encode_packet(&mut lzma_state, &mut enc, pkt);
        }

        // repair_remaining_packets
        self.repair_remaining(&mut lzma_state, &mut perplexity, packet_finder, &mut rng);
        self.perplexity = perplexity;
        true
    }

    fn mutate_next_packet(&mut self, lzma_state: &LZMAState, packet_finder: &mut TopKPacketFinder, rng: &mut impl Rng) -> bool {
        let pos = lzma_state.position;
        let packets = self.slab.packets();

        if pos + 1 < lzma_state.data.len() && rng.random_range(0..2u32) == 0 {
            let first = packets[pos];
            let second = packets[pos + 1];
            if (first.packet_type == LZMAPacketType::LongRep || first.packet_type == LZMAPacketType::Match) && first.len > 2 {
                self.undo_stack.insert(PacketSlabUndo { position: pos, old_packet: first });
                self.undo_stack.insert(PacketSlabUndo { position: pos + 1, old_packet: second });
                let mut new_second = first;
                new_second.len -= 1;
                let packets = self.slab.packets();
                packets[pos + 1] = new_second;
                packets[pos] = LZMAPacket::literal_packet();
                return true;
            } else if first.packet_type == LZMAPacketType::Literal || first.packet_type == LZMAPacketType::ShortRep {
                if second.packet_type == LZMAPacketType::Match || second.packet_type == LZMAPacketType::LongRep {
                    let rep_start = if second.packet_type == LZMAPacketType::LongRep {
                        pos.wrapping_sub(lzma_state.dists[second.dist as usize] as usize)
                    } else {
                        pos.wrapping_sub(second.dist as usize)
                    };
                    if second.len < 273 && rep_start > 0 && lzma_state.data[pos] == lzma_state.data[rep_start - 1] {
                        self.undo_stack.insert(PacketSlabUndo { position: pos, old_packet: first });
                        let mut new_first = second;
                        new_first.len += 1;
                        self.slab.packets()[pos] = new_first;
                        return true;
                    }
                }
            }
        }

        let old_packet = self.slab.packets()[pos];
        self.undo_stack.insert(PacketSlabUndo { position: pos, old_packet });
        self.pick_random_from_top_k(lzma_state, packet_finder, false, rng)
    }

    fn pick_random_from_top_k(&mut self, lzma_state: &LZMAState, packet_finder: &mut TopKPacketFinder, best: bool, rng: &mut impl Rng) -> bool {
        packet_finder.find(lzma_state, self.slab.packets());
        let count = packet_finder.count();
        if count == 0 { return false; }

        let mut choice = rand_max_dist(count, 8, rng);
        if rng.random_range(0..8u32) == 0 || best { choice = count - 1; }

        let mut idx = 0;
        while let Some(pkt) = packet_finder.pop() {
            if idx == choice {
                self.slab.packets()[lzma_state.position] = pkt;
                return true;
            }
            idx += 1;
        }
        false
    }

    fn repair_remaining(&mut self, lzma_state: &mut LZMAState, perplexity: &mut u64, packet_finder: &mut TopKPacketFinder, rng: &mut impl Rng) {
        let mut count = 0usize;
        while lzma_state.position < lzma_state.data.len() {
            count += 1;
            let pos = lzma_state.position;
            let old_packet = self.slab.packets()[pos];
            let mut packet = old_packet;

            if packet.packet_type == LZMAPacketType::ShortRep || packet.packet_type == LZMAPacketType::Literal {
                let rep_pos = pos.wrapping_sub(lzma_state.dists[0] as usize + 1);
                if rep_pos < lzma_state.data.len() && lzma_state.data[pos] == lzma_state.data[rep_pos] {
                    if count < 4 {
                        packet = LZMAPacket::short_rep_packet();
                    }
                } else {
                    packet = LZMAPacket::literal_packet();
                }
            }

            if packet.packet_type == LZMAPacketType::LongRep {
                let mut dist_index = 0u32;
                packet.dist = dist_index;
                while !validate_long_rep(lzma_state, &packet) && dist_index < 4 {
                    packet = LZMAPacket::long_rep_packet(dist_index, packet.len as u32);
                    dist_index += 1;
                }
                if !validate_long_rep(lzma_state, &packet) {
                    self.slab.packets()[pos] = packet;
                    let best = rng.random_range(0..4u32) == 0;
                    self.pick_random_from_top_k(lzma_state, packet_finder, best, rng);
                    packet = self.slab.packets()[pos];
                }
            }

            if !LZMAPacket::cmp(&old_packet, &packet) {
                self.undo_stack.insert(PacketSlabUndo { position: pos, old_packet });
                self.slab.packets()[pos] = packet;
            }

            let pkt = self.slab.packets()[pos];
            let mut enc = PerplexityEncoder { perplexity };
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

fn validate_long_rep(lzma_state: &LZMAState, packet: &LZMAPacket) -> bool {
    assert_eq!(packet.packet_type, LZMAPacketType::LongRep);
    if packet.dist >= 4 { return false; }
    let dist_val = lzma_state.dists[packet.dist as usize] as usize;
    if lzma_state.position < dist_val + 1 { return false; }
    let rep_start = lzma_state.position - dist_val - 1;
    let len = packet.len as usize;
    if rep_start + len > lzma_state.data.len() || lzma_state.position + len > lzma_state.data.len() { return false; }
    lzma_state.data[rep_start..rep_start + len] == lzma_state.data[lzma_state.position..lzma_state.position + len]
}

fn rand_max_dist(count: usize, num: usize, rng: &mut impl Rng) -> usize {
    let mut result = rng.random_range(0..count);
    for _ in 1..num {
        let r = rng.random_range(0..count);
        if r > result { result = r; }
    }
    result
}
