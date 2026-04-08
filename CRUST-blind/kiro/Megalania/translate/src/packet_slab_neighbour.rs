use crate::lzma_packet::{LZMAPacket, LZMAPacketType};
use crate::lzma_packet_encoder::lzma_encode_packet;
use crate::lzma_state::LZMAState;
use crate::packet_slab::PacketSlab;
use crate::packet_slab_undo_stack::{PacketSlabUndo, PacketSlabUndoStack};
use crate::perplexity_encoder::PerplexityEncoder;
use crate::top_k_packet_finder::TopKPacketFinder;

pub struct PacketSlabNeighbour<'a> {
    pub init_state: LZMAState<'a>,
    pub slab: Box<PacketSlab>,
    pub perplexity: u64,
    pub undo_stack: PacketSlabUndoStack,
}

fn encode_to_packet_number(lzma_state: &mut LZMAState, enc: &mut dyn crate::encoder_interface::EncoderInterface, packets: &[LZMAPacket], target: usize) {
    let mut count = 0;
    while lzma_state.position < lzma_state.data.len() {
        if count == target { break; }
        count += 1;
        let pkt = packets[lzma_state.position];
        lzma_encode_packet(lzma_state, enc, pkt);
    }
}

fn save_packet_at_position(undo_stack: &mut PacketSlabUndoStack, packet: LZMAPacket, position: usize) {
    undo_stack.insert(PacketSlabUndo { position, old_packet: packet });
}

fn rand_max_dist(count: usize, mut num: usize) -> usize {
    use rand::Rng;
    let mut rng = rand::rng();
    let mut rnd = rng.random_range(0..count);
    num -= 1;
    while num != 0 {
        rnd = rnd.max(rng.random_range(0..count));
        num -= 1;
    }
    rnd
}

fn pick_random_next_packet_from_top_k(
    lzma_state: &LZMAState,
    packet_finder: &mut TopKPacketFinder,
    packets: &mut [LZMAPacket],
    best: bool,
) -> bool {
    use rand::Rng;
    let mut rng = rand::rng();

    packet_finder.find(lzma_state, packets);
    let count = packet_finder.count();
    if count == 0 { return false; }

    let mut choice = rand_max_dist(count, 8);
    if rng.random_range(0u32..8) == 0 || best {
        choice = count - 1;
    }

    let pos = lzma_state.position;
    while let Some(pkt) = packet_finder.pop() {
        if choice == 0 {
            packets[pos] = pkt;
            return true;
        }
        choice -= 1;
    }
    true
}

fn validate_long_rep_packet(lzma_state: &LZMAState, packet: LZMAPacket) -> bool {
    let rep_start = lzma_state.position - lzma_state.dists[packet.dist as usize] as usize - 1;
    let len = packet.len as usize;
    lzma_state.data[rep_start..rep_start + len] == lzma_state.data[lzma_state.position..lzma_state.position + len]
}

fn repair_remaining_packets(
    undo_stack: &mut PacketSlabUndoStack,
    lzma_state: &mut LZMAState,
    enc: &mut dyn crate::encoder_interface::EncoderInterface,
    packet_finder: &mut TopKPacketFinder,
    packets: &mut [LZMAPacket],
) {
    use rand::Rng;
    let mut rng = rand::rng();
    let mut count = 0usize;

    while lzma_state.position < lzma_state.data.len() {
        count += 1;
        let pos = lzma_state.position;
        let old_packet = packets[pos];

        match packets[pos].packet_type {
            LZMAPacketType::ShortRep | LZMAPacketType::Literal => {
                if lzma_state.data[pos] == lzma_state.data[pos - lzma_state.dists[0] as usize - 1] {
                    if count < 4 {
                        packets[pos] = LZMAPacket::short_rep_packet();
                    }
                } else {
                    packets[pos] = LZMAPacket::literal_packet();
                }
            }
            LZMAPacketType::LongRep => {
                let mut dist_index = 0u32;
                while !validate_long_rep_packet(lzma_state, packets[pos]) && dist_index < 4 {
                    packets[pos].dist = dist_index;
                    dist_index += 1;
                }
                if !validate_long_rep_packet(lzma_state, packets[pos]) {
                    pick_random_next_packet_from_top_k(lzma_state, packet_finder, packets, rng.random_range(0u32..4) == 0);
                }
            }
            _ => {}
        }

        if !LZMAPacket::cmp(&old_packet, &packets[pos]) {
            save_packet_at_position(undo_stack, old_packet, pos);
        }

        let pkt = packets[pos];
        lzma_encode_packet(lzma_state, enc, pkt);
    }
}

fn mutate_next_packet(
    undo_stack: &mut PacketSlabUndoStack,
    lzma_state: &LZMAState,
    packet_finder: &mut TopKPacketFinder,
    packets: &mut [LZMAPacket],
) -> bool {
    use rand::Rng;
    let mut rng = rand::rng();
    let pos = lzma_state.position;

    // try to randomly grow/shrink the next match packet
    if pos + 1 < lzma_state.data.len() && rng.random_range(0u32..2) == 0 {
        let first = packets[pos];
        let second = packets[pos + 1];

        if (first.packet_type == LZMAPacketType::LongRep || first.packet_type == LZMAPacketType::Match) && first.len > 2 {
            save_packet_at_position(undo_stack, first, pos);
            save_packet_at_position(undo_stack, second, pos + 1);
            packets[pos + 1] = LZMAPacket { packet_type: first.packet_type, dist: first.dist, len: first.len - 1 };
            packets[pos] = LZMAPacket::literal_packet();
            return true;
        } else if first.packet_type == LZMAPacketType::Literal || first.packet_type == LZMAPacketType::ShortRep {
            if second.packet_type == LZMAPacketType::Match || second.packet_type == LZMAPacketType::LongRep {
                let rep_start = if second.packet_type == LZMAPacketType::LongRep {
                    pos - lzma_state.dists[second.dist as usize] as usize
                } else {
                    pos - second.dist as usize
                };
                if second.len < 273 && rep_start > 0 && lzma_state.data[pos] == lzma_state.data[rep_start - 1] {
                    save_packet_at_position(undo_stack, first, pos);
                    packets[pos] = LZMAPacket { packet_type: second.packet_type, dist: second.dist, len: second.len + 1 };
                    return true;
                }
            }
        }
    }

    save_packet_at_position(undo_stack, packets[pos], pos);
    pick_random_next_packet_from_top_k(lzma_state, packet_finder, packets, false)
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
    pub fn generate(&mut self, packet_finder: &mut TopKPacketFinder) -> bool {
        use rand::Rng;
        let mut rng = rand::rng();

        let packets = self.slab.packets();
        let packet_count = {
            let mut position = 0;
            let mut count = 0;
            while position < packets.len() {
                count += 1;
                position += packets[position].len as usize;
            }
            count
        };

        let mutation_target = rng.random_range(0..packet_count);

        let mut lzma_state = self.init_state.clone();
        self.perplexity = 0;
        let perplexity_ptr = &mut self.perplexity as *mut u64;

        // We need to work around borrow checker: use raw pointer for perplexity
        // since PerplexityEncoder borrows it mutably while we also need to access self.slab
        let enc_perplexity = unsafe { &mut *perplexity_ptr };
        let mut enc = PerplexityEncoder { perplexity: enc_perplexity };

        let packets = self.slab.packets();
        encode_to_packet_number(&mut lzma_state, &mut enc, packets, mutation_target);

        if !mutate_next_packet(&mut self.undo_stack, &lzma_state, packet_finder, packets) {
            return false;
        }

        let pkt = packets[lzma_state.position];
        lzma_encode_packet(&mut lzma_state, &mut enc, pkt);

        repair_remaining_packets(&mut self.undo_stack, &mut lzma_state, &mut enc, packet_finder, packets);
        true
    }
    pub fn undo(&mut self) {
        self.undo_stack.apply(&mut self.slab);
    }
    pub fn undo_count(&self) -> usize {
        self.undo_stack.count()
    }
}
