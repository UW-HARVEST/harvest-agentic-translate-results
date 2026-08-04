use crate::encoder_interface::EncoderInterface;
use crate::lzma_packet::LZMAPacket;
use crate::lzma_packet::LZMAPacketType;
use crate::lzma_packet_encoder::lzma_encode_packet;
use crate::lzma_state::LZMAState;
use crate::packet_slab::PacketSlab;
use crate::packet_slab_undo_stack::PacketSlabUndo;
use crate::packet_slab_undo_stack::PacketSlabUndoStack;
use crate::perplexity_table::LOG2_LOOKUP;
use crate::probability::Prob;
use crate::top_k_packet_finder::TopKPacketFinder;
pub struct PacketSlabNeighbour<'a> {
    pub init_state: LZMAState<'a>,
    pub slab: Box<PacketSlab>,
    pub perplexity: u64,
    pub undo_stack: PacketSlabUndoStack,
}
impl<'a> PacketSlabNeighbour<'a> {
    pub fn new(slab: PacketSlab, init_state: LZMAState) -> Self {
        let init_state: LZMAState<'a> = unsafe { std::mem::transmute(init_state) };
        Self {
            init_state,
            slab: Box::new(slab),
            perplexity: 0,
            undo_stack: PacketSlabUndoStack::new(),
        }
    }
    pub fn free(self) {
        drop(self);
    }
    pub fn generate(&mut self, packet_finder: &mut TopKPacketFinder) -> bool {
        self.undo_stack = PacketSlabUndoStack::new();
        let packet_count = self.slab.count();
        if packet_count == 0 {
            return false;
        }

        self.perplexity = 0;
        let original_packets = self.slab.packets().to_vec();
        let mut packets = original_packets.clone();
        let mut lzma_state = self.init_state.clone();
        let mutation_target = random_bounded(packet_count);
        let mut encoder = PerplexityCounter {
            perplexity: &mut self.perplexity,
        };

        encode_to_packet_number(&mut lzma_state, &mut encoder, &packets, mutation_target);
        if !mutate_next_packet(&lzma_state, packet_finder, &mut packets) {
            return false;
        }
        let packet = packets[lzma_state.position];
        lzma_encode_packet(&mut lzma_state, &mut encoder, packet);
        repair_remaining_packets(&mut lzma_state, &mut encoder, packet_finder, &mut packets);

        let slab_packets = self.slab.packets();
        for (position, (old_packet, new_packet)) in original_packets
            .iter()
            .copied()
            .zip(packets.iter().copied())
            .enumerate()
        {
            if !LZMAPacket::cmp(&old_packet, &new_packet) {
                self.undo_stack.insert(PacketSlabUndo {
                    position,
                    old_packet,
                });
                slab_packets[position] = new_packet;
            }
        }
        true
    }
    pub fn undo(&mut self) {
        self.undo_stack.apply(&mut self.slab);
    }
    pub fn undo_count(&self) -> usize {
        self.undo_stack.count()
    }
}

fn random_bounded(bound: usize) -> usize {
    if bound == 0 {
        0
    } else {
        (rand::random::<u64>() % bound as u64) as usize
    }
}

fn encode_to_packet_number(
    lzma_state: &mut LZMAState,
    enc: &mut dyn EncoderInterface,
    packets: &[LZMAPacket],
    target: usize,
) {
    let mut count = 0usize;
    while lzma_state.position < lzma_state.data.len() {
        if count == target {
            break;
        }
        count += 1;
        lzma_encode_packet(lzma_state, enc, packets[lzma_state.position]);
    }
}

fn rand_max_dist(count: usize, num: usize) -> usize {
    let mut rnd = random_bounded(count);
    let mut remaining = num;
    while remaining > 1 {
        rnd = rnd.max(random_bounded(count));
        remaining -= 1;
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
    if random_bounded(8) == 0 || best {
        choice = count - 1;
    }

    while let Some(packet) = packet_finder.pop() {
        packets[lzma_state.position] = packet;
        if choice == 0 {
            return true;
        }
        choice -= 1;
    }
    true
}

fn validate_long_rep_packet(lzma_state: &LZMAState, packet: LZMAPacket) -> bool {
    if packet.packet_type != LZMAPacketType::LongRep {
        return false;
    }
    let dist = lzma_state.dists[packet.dist as usize] as usize;
    if lzma_state.position <= dist {
        return false;
    }
    let rep_start = lzma_state.position - dist - 1;
    let end = lzma_state.position + packet.len as usize;
    end <= lzma_state.data.len()
        && lzma_state.data[rep_start..rep_start + packet.len as usize]
            == lzma_state.data[lzma_state.position..end]
}

fn repair_remaining_packets(
    lzma_state: &mut LZMAState,
    enc: &mut dyn EncoderInterface,
    packet_finder: &mut TopKPacketFinder,
    packets: &mut [LZMAPacket],
) {
    let mut count = 0usize;
    while lzma_state.position < lzma_state.data.len() {
        count += 1;
        let position = lzma_state.position;
        let mut packet = packets[position];

        if matches!(
            packet.packet_type,
            LZMAPacketType::ShortRep | LZMAPacketType::Literal
        ) {
            if position > lzma_state.dists[0] as usize
                && lzma_state.data[position]
                    == lzma_state.data[position - lzma_state.dists[0] as usize - 1]
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
                packets[position] = packet;
                pick_random_next_packet_from_top_k(
                    lzma_state,
                    packet_finder,
                    packets,
                    random_bounded(4) == 0,
                );
                packet = packets[position];
            }
        }

        packets[position] = packet;
        lzma_encode_packet(lzma_state, enc, packet);
    }
}

fn mutate_next_packet(
    lzma_state: &LZMAState,
    packet_finder: &mut TopKPacketFinder,
    packets: &mut [LZMAPacket],
) -> bool {
    if lzma_state.position + 1 < lzma_state.data.len() && random_bounded(2) == 0 {
        let first_packet = packets[lzma_state.position];
        let second_packet = packets[lzma_state.position + 1];

        if matches!(first_packet.packet_type, LZMAPacketType::LongRep | LZMAPacketType::Match)
            && first_packet.len > 2
        {
            packets[lzma_state.position + 1] = LZMAPacket {
                len: first_packet.len - 1,
                ..first_packet
            };
            packets[lzma_state.position] = LZMAPacket::literal_packet();
            return true;
        } else if matches!(
            first_packet.packet_type,
            LZMAPacketType::Literal | LZMAPacketType::ShortRep
        ) && matches!(second_packet.packet_type, LZMAPacketType::Match | LZMAPacketType::LongRep)
        {
            let rep_start = if second_packet.packet_type == LZMAPacketType::LongRep {
                lzma_state
                    .position
                    .checked_sub(lzma_state.dists[second_packet.dist as usize] as usize)
            } else {
                lzma_state.position.checked_sub(second_packet.dist as usize)
            };

            if let Some(rep_start) = rep_start {
                if second_packet.len < 273
                    && rep_start > 0
                    && lzma_state.data[lzma_state.position] == lzma_state.data[rep_start - 1]
                {
                    packets[lzma_state.position] = LZMAPacket {
                        len: second_packet.len + 1,
                        ..second_packet
                    };
                    return true;
                }
            }
        }
    }

    pick_random_next_packet_from_top_k(lzma_state, packet_finder, packets, false)
}

struct PerplexityCounter<'a> {
    perplexity: &'a mut u64,
}

impl EncoderInterface for PerplexityCounter<'_> {
    fn encode_bit(&mut self, bit: bool, prob: Prob) {
        *self.perplexity += LOG2_LOOKUP[if bit {
            (2048 - prob) as usize
        } else {
            prob as usize
        }];
    }

    fn encode_direct_bits(&mut self, _bits: u32, num_bits: u32) {
        *self.perplexity += (num_bits as u64) << 11;
    }
}
