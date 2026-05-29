use crate::encoder_interface::EncoderInterface;
use crate::lzma_packet::{LZMAPacket, LZMAPacketType};
use crate::lzma_packet_encoder::lzma_encode_packet;
use crate::lzma_state::LZMAState;
use crate::packet_slab::PacketSlab;
use crate::packet_slab_undo_stack::{PacketSlabUndo, PacketSlabUndoStack};
use crate::probability::Prob;
use crate::top_k_packet_finder::TopKPacketFinder;
use rand::Rng;

pub struct PacketSlabNeighbour<'a> {
    pub init_state: LZMAState<'a>,
    pub slab: Box<PacketSlab>,
    pub perplexity: u64,
    pub undo_stack: PacketSlabUndoStack,
}

/// A perplexity-tracking encoder used internally during neighbour generation.
struct PerplexityEnc<'a> {
    perplexity: &'a mut u64,
}

impl<'a> EncoderInterface for PerplexityEnc<'a> {
    fn encode_bit(&mut self, bit: bool, prob: Prob) {
        let idx = if bit {
            (2048u32 - prob as u32) as usize
        } else {
            prob as usize
        };
        *self.perplexity += crate::perplexity_table::LOG2_LOOKUP[idx];
    }

    fn encode_direct_bits(&mut self, _bits: u32, num_bits: u32) {
        *self.perplexity += (num_bits as u64) << 11;
    }
}

fn rand_max_dist(count: usize, num: usize) -> usize {
    let mut rng = rand::rng();
    let mut rnd = rng.random_range(0..count);
    let mut n = num;
    loop {
        n -= 1;
        if n == 0 {
            break;
        }
        let next = rng.random_range(0..count);
        if next > rnd {
            rnd = next;
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
    let mut choice = rand_max_dist(count, 8);
    let mut rng = rand::rng();
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
    debug_assert_eq!(packet.packet_type, LZMAPacketType::LongRep);
    let dist_idx = packet.dist as usize;
    if dist_idx >= 4 {
        return false;
    }
    let dist = lzma_state.dists[dist_idx] as usize;
    let pos = lzma_state.position;
    if dist + 1 > pos {
        return false;
    }
    let rep_start = pos - dist - 1;
    let len = packet.len as usize;
    if pos + len > lzma_state.data.len() || rep_start + len > lzma_state.data.len() {
        return false;
    }
    lzma_state.data[rep_start..rep_start + len] == lzma_state.data[pos..pos + len]
}

fn encode_to_packet_number<'a>(
    lzma_state: &mut LZMAState<'a>,
    enc: &mut dyn EncoderInterface,
    packets: &[LZMAPacket],
    target: usize,
) {
    let mut count = 0;
    while lzma_state.position < lzma_state.data.len() {
        if count == target {
            break;
        }
        count += 1;
        let packet = packets[lzma_state.position];
        lzma_encode_packet(lzma_state, enc, packet);
    }
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
        // Drop will handle resource cleanup automatically.
    }

    pub fn generate(&mut self, packet_finder: &mut TopKPacketFinder) -> bool {
        let packet_count = self.slab.count();
        if packet_count == 0 {
            return false;
        }
        let mut rng = rand::rng();
        let mutation_target = rng.random_range(0..packet_count);

        // Take a working copy of the initial state to mutate.
        let mut lzma_state = self.init_state.clone();
        self.perplexity = 0;

        // Encode up to the mutation target.
        {
            let mut enc = PerplexityEnc {
                perplexity: &mut self.perplexity,
            };
            let packets: &[LZMAPacket] = self.slab.packets();
            encode_to_packet_number(&mut lzma_state, &mut enc, packets, mutation_target);
        }

        // Mutate the next packet.
        if !self.mutate_next_packet(&lzma_state, packet_finder) {
            return false;
        }

        // Encode the mutated packet.
        {
            let mut enc = PerplexityEnc {
                perplexity: &mut self.perplexity,
            };
            let packet = self.slab.packets()[lzma_state.position];
            lzma_encode_packet(&mut lzma_state, &mut enc, packet);
        }

        // Repair the remaining packets.
        self.repair_remaining_packets(&mut lzma_state, packet_finder);
        true
    }

    fn save_packet_at_position(&mut self, packet: LZMAPacket, position: usize) {
        let undo = PacketSlabUndo {
            position,
            old_packet: packet,
        };
        self.undo_stack.insert(undo);
    }

    fn mutate_next_packet(
        &mut self,
        lzma_state: &LZMAState,
        packet_finder: &mut TopKPacketFinder,
    ) -> bool {
        let pos = lzma_state.position;
        let data_size = lzma_state.data.len();
        let mut rng = rand::rng();

        let packets = self.slab.packets();

        if pos + 1 < data_size && rng.random_range(0..2) == 0 {
            let first_packet = packets[pos];
            let second_packet = packets[pos + 1];
            if (first_packet.packet_type == LZMAPacketType::LongRep
                || first_packet.packet_type == LZMAPacketType::Match)
                && first_packet.len > 2
            {
                self.save_packet_at_position(first_packet, pos);
                self.save_packet_at_position(second_packet, pos + 1);
                let packets = self.slab.packets();
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
                        let idx = second_packet.dist as usize;
                        if idx < 4 {
                            let d = lzma_state.dists[idx] as usize;
                            if d <= pos {
                                pos.checked_sub(d)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        let d = second_packet.dist as usize;
                        if d <= pos {
                            pos.checked_sub(d)
                        } else {
                            None
                        }
                    };
                    if let Some(rep_start) = rep_start {
                        if second_packet.len < 273
                            && rep_start > 0
                            && lzma_state.data[pos] == lzma_state.data[rep_start - 1]
                        {
                            self.save_packet_at_position(first_packet, pos);
                            let packets = self.slab.packets();
                            let mut new_first = second_packet;
                            new_first.len += 1;
                            packets[pos] = new_first;
                            return true;
                        }
                    }
                }
            }
        }

        // Default: replace with a top-k pick.
        let saved = self.slab.packets()[pos];
        self.save_packet_at_position(saved, pos);
        let packets = self.slab.packets();
        if pick_random_next_packet_from_top_k(lzma_state, packet_finder, packets, false) {
            return true;
        }
        false
    }

    fn repair_remaining_packets(
        &mut self,
        lzma_state: &mut LZMAState<'a>,
        packet_finder: &mut TopKPacketFinder,
    ) {
        let mut count = 0;
        let data_size = lzma_state.data.len();
        let mut rng = rand::rng();
        while lzma_state.position < data_size {
            count += 1;
            let pos = lzma_state.position;
            let old_packet = self.slab.packets()[pos];
            let mut new_packet = old_packet;

            if new_packet.packet_type == LZMAPacketType::ShortRep
                || new_packet.packet_type == LZMAPacketType::Literal
            {
                let dist0 = lzma_state.dists[0] as usize;
                if dist0 + 1 <= pos
                    && lzma_state.data[pos] == lzma_state.data[pos - dist0 - 1]
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
                    let best = rng.random_range(0..4) == 0;
                    self.slab.packets()[pos] = new_packet;
                    let packets = self.slab.packets();
                    pick_random_next_packet_from_top_k(lzma_state, packet_finder, packets, best);
                    new_packet = self.slab.packets()[pos];
                }
            }

            // Write the (possibly new) packet back into the slab.
            self.slab.packets()[pos] = new_packet;

            if !LZMAPacket::cmp(&old_packet, &new_packet) {
                self.save_packet_at_position(old_packet, pos);
            }

            // Encode it.
            {
                let mut enc = PerplexityEnc {
                    perplexity: &mut self.perplexity,
                };
                lzma_encode_packet(lzma_state, &mut enc, new_packet);
            }
        }
    }

    pub fn undo(&mut self) {
        self.undo_stack.apply(&mut self.slab);
    }

    pub fn undo_count(&self) -> usize {
        self.undo_stack.count()
    }
}
