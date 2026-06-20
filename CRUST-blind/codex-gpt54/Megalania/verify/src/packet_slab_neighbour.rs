use crate::lzma_state::LZMAState;
use crate::lzma_packet::{LZMAPacket, LZMAPacketType};
use crate::packet_slab::PacketSlab;
use crate::packet_slab_undo_stack::PacketSlabUndo;
use crate::packet_slab_undo_stack::PacketSlabUndoStack;
use crate::top_k_packet_finder::TopKPacketFinder;
use rand::Rng;

struct DummyEncoder {
    _tag: u8,
}

impl crate::encoder_interface::EncoderInterface for DummyEncoder {
    fn encode_bit(&mut self, _bit: bool, _prob: crate::probability::Prob) {}
    fn encode_direct_bits(&mut self, _bits: u32, _num_bits: u32) {}
}

fn encode_to_packet_number(
    lzma_state: &mut LZMAState,
    enc: &mut dyn crate::encoder_interface::EncoderInterface,
    packets: &[LZMAPacket],
    target: usize,
) {
    let mut count = 0usize;
    while lzma_state.position < lzma_state.data.len() {
        if count == target {
            break;
        }
        count += 1;
        crate::lzma_packet_encoder::lzma_encode_packet(lzma_state, enc, packets[lzma_state.position]);
    }
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

fn rand_max_dist<R: Rng>(count: usize, num: usize, rng: &mut R) -> usize {
    let mut rnd = rng.random_range(0..count);
    for _ in 1..num {
        rnd = rnd.max(rng.random_range(0..count));
    }
    rnd
}

fn pick_random_next_packet_from_top_k<R: Rng>(
    lzma_state: &LZMAState,
    packet_finder: &mut TopKPacketFinder,
    packets: &mut [LZMAPacket],
    best: bool,
    rng: &mut R,
) -> bool {
    packet_finder.find(lzma_state, packets);
    let count = packet_finder.count();
    if count == 0 {
        return false;
    }
    let mut choice = rand_max_dist(count, 8, rng);
    if rng.random_range(0..8) == 0 || best {
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

fn validate_long_rep_packet(lzma_state: &LZMAState, packet: LZMAPacket) -> bool {
    if packet.packet_type != LZMAPacketType::LongRep {
        return false;
    }
    let dist_index = packet.dist as usize;
    if dist_index >= lzma_state.dists.len() || lzma_state.position <= lzma_state.dists[dist_index] as usize {
        return false;
    }
    let start = lzma_state.position - lzma_state.dists[dist_index] as usize - 1;
    let end = lzma_state.position + packet.len as usize;
    end <= lzma_state.data.len()
        && lzma_state.data[start..start + packet.len as usize]
            == lzma_state.data[lzma_state.position..end]
}

fn repair_remaining_packets<R: Rng>(
    undo_stack: &mut PacketSlabUndoStack,
    lzma_state: &mut LZMAState,
    enc: &mut dyn crate::encoder_interface::EncoderInterface,
    packet_finder: &mut TopKPacketFinder,
    packets: &mut [LZMAPacket],
    rng: &mut R,
) {
    let mut count = 0usize;
    while lzma_state.position < lzma_state.data.len() {
        count += 1;
        let position = lzma_state.position;
        let old_packet = packets[position];
        let mut updated_packet = old_packet;

        if matches!(updated_packet.packet_type, LZMAPacketType::ShortRep | LZMAPacketType::Literal) {
            if position > lzma_state.dists[0] as usize
                && lzma_state.data[position]
                    == lzma_state.data[position - lzma_state.dists[0] as usize - 1]
            {
                if count < 4 {
                    updated_packet = LZMAPacket::short_rep_packet();
                }
            } else {
                updated_packet = LZMAPacket::literal_packet();
            }
        }

        if updated_packet.packet_type == LZMAPacketType::LongRep {
            let mut dist_index = 0u32;
            while !validate_long_rep_packet(lzma_state, updated_packet) && dist_index < 4 {
                updated_packet.dist = dist_index;
                dist_index += 1;
            }
            if !validate_long_rep_packet(lzma_state, updated_packet) {
                let _ = pick_random_next_packet_from_top_k(
                    lzma_state,
                    packet_finder,
                    packets,
                    rng.random_range(0..4) == 0,
                    rng,
                );
                updated_packet = packets[position];
            }
        }

        if !LZMAPacket::cmp(&old_packet, &updated_packet) {
            save_packet_at_position(undo_stack, old_packet, position);
        }
        packets[position] = updated_packet;
        crate::lzma_packet_encoder::lzma_encode_packet(lzma_state, enc, updated_packet);
    }
}

fn mutate_next_packet<R: Rng>(
    undo_stack: &mut PacketSlabUndoStack,
    lzma_state: &LZMAState,
    packet_finder: &mut TopKPacketFinder,
    packets: &mut [LZMAPacket],
    rng: &mut R,
) -> bool {
    let position = lzma_state.position;
    let first_packet = packets[position];
    if position + 1 < lzma_state.data.len() && rng.random_range(0..2) == 0 {
        let second_packet = packets[position + 1];
        if matches!(first_packet.packet_type, LZMAPacketType::LongRep | LZMAPacketType::Match)
            && first_packet.len > 2
        {
            save_packet_at_position(undo_stack, first_packet, position);
            save_packet_at_position(undo_stack, second_packet, position + 1);
            packets[position + 1] = first_packet;
            packets[position + 1].len -= 1;
            packets[position] = LZMAPacket::literal_packet();
            return true;
        } else if matches!(first_packet.packet_type, LZMAPacketType::Literal | LZMAPacketType::ShortRep)
            && matches!(second_packet.packet_type, LZMAPacketType::Match | LZMAPacketType::LongRep)
        {
            let rep_start = if second_packet.packet_type == LZMAPacketType::LongRep {
                let idx = second_packet.dist as usize;
                if idx >= lzma_state.dists.len() {
                    0usize
                } else {
                    position.saturating_sub(lzma_state.dists[idx] as usize)
                }
            } else {
                position.saturating_sub(second_packet.dist as usize)
            };

            if second_packet.len < 273
                && rep_start > 0
                && lzma_state.data[position] == lzma_state.data[rep_start - 1]
            {
                save_packet_at_position(undo_stack, first_packet, position);
                packets[position] = second_packet;
                packets[position].len += 1;
                return true;
            }
        }
    }

    save_packet_at_position(undo_stack, packets[position], position);
    pick_random_next_packet_from_top_k(lzma_state, packet_finder, packets, false, rng)
}

pub struct PacketSlabNeighbour<'a> {
    pub init_state: LZMAState<'a>,
    pub slab: Box<PacketSlab>,
    pub perplexity: u64,
    pub undo_stack: PacketSlabUndoStack,
}
impl<'a> PacketSlabNeighbour<'a> {
    pub fn new(slab: PacketSlab, init_state: LZMAState) -> Self {
        // SAFETY: the neighbour stores the same borrowed state data the caller supplied.
        let init_state = unsafe { std::mem::transmute::<LZMAState<'_>, LZMAState<'a>>(init_state) };
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
        let packet_count = self.slab.count();
        if packet_count == 0 {
            return false;
        }

        let mut rng = rand::rng();
        let packets = self.slab.packets();
        let mut lzma_state = self.init_state.clone();
        let mut enc = DummyEncoder { _tag: 0 };
        self.perplexity = 0;
        crate::perplexity_encoder::perplexity_encoder_new(&mut enc, &mut self.perplexity);

        let mutation_target = rng.random_range(0..packet_count);
        encode_to_packet_number(&mut lzma_state, &mut enc, packets, mutation_target);
        if !mutate_next_packet(&mut self.undo_stack, &lzma_state, packet_finder, packets, &mut rng) {
            return false;
        }
        let packet = packets[lzma_state.position];
        crate::lzma_packet_encoder::lzma_encode_packet(&mut lzma_state, &mut enc, packet);
        repair_remaining_packets(
            &mut self.undo_stack,
            &mut lzma_state,
            &mut enc,
            packet_finder,
            packets,
            &mut rng,
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
