use crate::encoder_interface::EncoderInterface;
use crate::lzma_packet::{LZMAPacket, LZMAPacketType};
use crate::lzma_packet_encoder::lzma_encode_packet;
use crate::lzma_state::LZMAState;
use crate::packet_slab::PacketSlab;
use crate::packet_slab_undo_stack::{PacketSlabUndo, PacketSlabUndoStack};
use crate::perplexity_table::LOG2_LOOKUP;
use crate::probability::Prob;
use crate::top_k_packet_finder::TopKPacketFinder;
use rand::Rng;

const DECIMAL_PLACES: u32 = 11;

/// Local perplexity encoder used internally by the neighbour-generation logic.
struct LocalPerplexityEncoder<'a> {
    perplexity: &'a mut u64,
}

impl<'a> EncoderInterface for LocalPerplexityEncoder<'a> {
    fn encode_bit(&mut self, bit: bool, prob: Prob) {
        let idx = if bit {
            (2048 - prob as usize) as usize
        } else {
            prob as usize
        };
        *self.perplexity += LOG2_LOOKUP[idx];
    }

    fn encode_direct_bits(&mut self, _bits: u32, num_bits: u32) {
        *self.perplexity += (num_bits as u64) << DECIMAL_PLACES;
    }
}

pub struct PacketSlabNeighbour<'a> {
    pub init_state: LZMAState<'a>,
    pub slab: Box<PacketSlab>,
    pub perplexity: u64,
    pub undo_stack: PacketSlabUndoStack,
}

fn save_packet_at_position(
    undo_stack: &mut PacketSlabUndoStack,
    packet: LZMAPacket,
    position: usize,
) {
    undo_stack.insert(PacketSlabUndo {
        position,
        old_packet: packet,
    });
}

fn rand_max_dist<R: Rng>(rng: &mut R, count: usize, mut num: usize) -> usize {
    let mut rnd = rng.random_range(0..count);
    while num > 1 {
        let next = rng.random_range(0..count);
        if next > rnd {
            rnd = next;
        }
        num -= 1;
    }
    rnd
}

fn validate_long_rep_packet(lzma_state: &LZMAState, packet: LZMAPacket) -> bool {
    debug_assert!(matches!(packet.packet_type, LZMAPacketType::LongRep));
    let dist_index = packet.dist as usize;
    if dist_index >= 4 {
        return false;
    }
    let dist = lzma_state.dists[dist_index] as usize;
    if lzma_state.position == 0 {
        return false;
    }
    if lzma_state.position <= dist {
        return false;
    }
    let rep_start = lzma_state.position - dist - 1;
    let len = packet.len as usize;
    if rep_start + len > lzma_state.data.len() {
        return false;
    }
    if lzma_state.position + len > lzma_state.data.len() {
        return false;
    }
    for i in 0..len {
        if lzma_state.data[rep_start + i] != lzma_state.data[lzma_state.position + i] {
            return false;
        }
    }
    true
}

fn encode_to_packet_number(
    lzma_state: &mut LZMAState,
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

fn pick_random_next_packet_from_top_k<R: Rng>(
    rng: &mut R,
    lzma_state: &LZMAState,
    packet_finder: &mut TopKPacketFinder,
    packets: &mut [LZMAPacket],
    best: bool,
) -> bool {
    let pos = lzma_state.position;
    packet_finder.find(lzma_state, packets);
    let count = packet_finder.count();
    if count == 0 {
        return false;
    }
    let mut choice = rand_max_dist(rng, count, 8);
    if rng.random_range(0..8u32) == 0 || best {
        choice = count - 1;
    }
    while let Some(packet) = packet_finder.pop() {
        if choice == 0 {
            packets[pos] = packet;
            return true;
        }
        choice -= 1;
    }
    true
}

fn mutate_next_packet<R: Rng>(
    rng: &mut R,
    undo_stack: &mut PacketSlabUndoStack,
    lzma_state: &LZMAState,
    packet_finder: &mut TopKPacketFinder,
    packets: &mut [LZMAPacket],
) -> bool {
    let pos = lzma_state.position;
    let data_len = lzma_state.data.len();

    if pos + 1 < data_len && rng.random_range(0..2u32) == 0 {
        let first_packet = packets[pos];
        let second_packet = packets[pos + 1];
        match first_packet.packet_type {
            LZMAPacketType::LongRep | LZMAPacketType::Match if first_packet.len > 2 => {
                save_packet_at_position(undo_stack, first_packet, pos);
                save_packet_at_position(undo_stack, second_packet, pos + 1);
                let mut new_second = first_packet;
                new_second.len -= 1;
                packets[pos + 1] = new_second;
                packets[pos] = LZMAPacket::literal_packet();
                return true;
            }
            LZMAPacketType::Literal | LZMAPacketType::ShortRep => {
                if matches!(
                    second_packet.packet_type,
                    LZMAPacketType::Match | LZMAPacketType::LongRep
                ) {
                    let rep_start = match second_packet.packet_type {
                        LZMAPacketType::LongRep => {
                            let dist_idx = second_packet.dist as usize;
                            if dist_idx >= 4 {
                                None
                            } else {
                                let d = lzma_state.dists[dist_idx] as usize;
                                if pos > d {
                                    Some(pos - d)
                                } else {
                                    None
                                }
                            }
                        }
                        LZMAPacketType::Match => {
                            let d = second_packet.dist as usize;
                            if pos > d {
                                Some(pos - d)
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };
                    if let Some(rep_start) = rep_start {
                        if second_packet.len < 273
                            && rep_start > 0
                            && lzma_state.data[pos] == lzma_state.data[rep_start - 1]
                        {
                            save_packet_at_position(undo_stack, packets[pos], pos);
                            let mut new_first = second_packet;
                            new_first.len += 1;
                            packets[pos] = new_first;
                            return true;
                        }
                    }
                }
            }
            _ => {}
        }
    }
    save_packet_at_position(undo_stack, packets[pos], pos);
    pick_random_next_packet_from_top_k(rng, lzma_state, packet_finder, packets, false)
}

fn repair_remaining_packets<R: Rng>(
    rng: &mut R,
    undo_stack: &mut PacketSlabUndoStack,
    lzma_state: &mut LZMAState,
    enc: &mut dyn EncoderInterface,
    packet_finder: &mut TopKPacketFinder,
    packets: &mut [LZMAPacket],
) {
    let mut count: usize = 0;
    while lzma_state.position < lzma_state.data.len() {
        count += 1;
        let pos = lzma_state.position;
        let mut packet = packets[pos];
        let old_packet = packet;

        // SHORT_REP / LITERAL repair.
        if matches!(
            packet.packet_type,
            LZMAPacketType::ShortRep | LZMAPacketType::Literal
        ) {
            let dist0 = lzma_state.dists[0] as usize;
            let valid = pos > dist0
                && lzma_state.data[pos] == lzma_state.data[pos - dist0 - 1];
            if valid {
                if count < 4 {
                    packet = LZMAPacket::short_rep_packet();
                }
            } else {
                packet = LZMAPacket::literal_packet();
            }
        }

        // LONG_REP repair.
        if matches!(packet.packet_type, LZMAPacketType::LongRep) {
            let mut dist_index: u32 = 0;
            while !validate_long_rep_packet(lzma_state, packet) && dist_index < 4 {
                packet.dist = dist_index;
                dist_index += 1;
            }
            if !validate_long_rep_packet(lzma_state, packet) {
                packets[pos] = packet;
                let best = rng.random_range(0..4u32) == 0;
                pick_random_next_packet_from_top_k(
                    rng,
                    lzma_state,
                    packet_finder,
                    packets,
                    best,
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

impl<'a> PacketSlabNeighbour<'a> {
    pub fn new(slab: PacketSlab, init_state: LZMAState<'a>) -> Self {
        PacketSlabNeighbour {
            init_state,
            slab: Box::new(slab),
            perplexity: 0,
            undo_stack: PacketSlabUndoStack::new(),
        }
    }
    /// Consumes the neighbour. Equivalent to the C `packet_slab_neighbour_free`.
    pub fn free(self) {
        // Resources are released automatically when `self` is dropped.
    }
    pub fn generate(&mut self, packet_finder: &mut TopKPacketFinder) -> bool {
        let mut rng = rand::rng();

        let packet_count = self.slab.count();
        if packet_count == 0 {
            return false;
        }
        let mutation_target = rng.random_range(0..packet_count);

        let mut lzma_state = self.init_state.clone();
        self.perplexity = 0;
        let mut perp = 0u64;

        // First, encode up to the mutation target without modifying anything.
        {
            let mut enc = LocalPerplexityEncoder {
                perplexity: &mut perp,
            };
            let packets = self.slab.packets();
            encode_to_packet_number(&mut lzma_state, &mut enc, packets, mutation_target);
        }

        // Mutate the next packet.
        {
            let packets = self.slab.packets();
            if !mutate_next_packet(
                &mut rng,
                &mut self.undo_stack,
                &lzma_state,
                packet_finder,
                packets,
            ) {
                self.perplexity = perp;
                return false;
            }
        }

        // Encode the mutated packet.
        {
            let mut enc = LocalPerplexityEncoder {
                perplexity: &mut perp,
            };
            let packets = self.slab.packets();
            let packet = packets[lzma_state.position];
            lzma_encode_packet(&mut lzma_state, &mut enc, packet);
        }

        // Repair / encode the remaining packets.
        {
            let mut enc = LocalPerplexityEncoder {
                perplexity: &mut perp,
            };
            let packets = self.slab.packets();
            repair_remaining_packets(
                &mut rng,
                &mut self.undo_stack,
                &mut lzma_state,
                &mut enc,
                packet_finder,
                packets,
            );
        }

        self.perplexity = perp;
        true
    }
    pub fn undo(&mut self) {
        self.undo_stack.apply(&mut self.slab);
    }
    pub fn undo_count(&self) -> usize {
        self.undo_stack.count()
    }
}
