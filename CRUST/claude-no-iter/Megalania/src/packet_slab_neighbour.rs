use crate::encoder_interface::EncoderInterface;
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
    let mut rnd: usize = rng.gen_range(0..count);
    let mut remaining = num;
    while remaining > 1 {
        let next: usize = rng.gen_range(0..count);
        if next > rnd {
            rnd = next;
        }
        remaining -= 1;
    }
    rnd
}

fn validate_long_rep_packet(lzma_state: &LZMAState, packet: LZMAPacket) -> bool {
    assert!(packet.packet_type == LZMAPacketType::LongRep);
    let dist = lzma_state.dists[packet.dist as usize] as usize;
    if lzma_state.position < dist + 1 {
        return false;
    }
    let rep_start = lzma_state.position - dist - 1;
    let len = packet.len as usize;
    if rep_start + len > lzma_state.data.len() || lzma_state.position + len > lzma_state.data.len()
    {
        return false;
    }
    lzma_state.data[rep_start..rep_start + len]
        == lzma_state.data[lzma_state.position..lzma_state.position + len]
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

/// Encodes packets up to a target packet number.
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
        let packet = packets[lzma_state.position];
        lzma_encode_packet(lzma_state, enc, packet);
    }
}

/// Pick a random next packet from the top-K finder. Mutates `packets[pos]`.
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
    let mut rng = rand::thread_rng();
    if rng.gen_range(0..8) == 0 || best {
        choice = count - 1;
    }
    let pos = lzma_state.position;
    while let Some(p) = packet_finder.pop() {
        if choice == 0 {
            packets[pos] = p;
            return true;
        }
        choice -= 1;
    }
    true
}

fn mutate_next_packet(
    undo_stack: &mut PacketSlabUndoStack,
    lzma_state: &LZMAState,
    packet_finder: &mut TopKPacketFinder,
    packets: &mut [LZMAPacket],
) -> bool {
    let mut rng = rand::thread_rng();
    let pos = lzma_state.position;
    if pos + 1 < lzma_state.data.len() && rng.gen_range(0..2) == 0 {
        let first_packet = packets[pos];
        let second_packet = packets[pos + 1];
        if (first_packet.packet_type == LZMAPacketType::LongRep
            || first_packet.packet_type == LZMAPacketType::Match)
            && first_packet.len > 2
        {
            save_packet_at_position(undo_stack, first_packet, pos);
            save_packet_at_position(undo_stack, second_packet, pos + 1);
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
                let rep_start: i64 = if second_packet.packet_type == LZMAPacketType::LongRep {
                    pos as i64 - lzma_state.dists[second_packet.dist as usize] as i64
                } else {
                    pos as i64 - second_packet.dist as i64
                };
                if second_packet.len < 273
                    && rep_start > 0
                    && lzma_state.data[pos] == lzma_state.data[(rep_start - 1) as usize]
                {
                    save_packet_at_position(undo_stack, first_packet, pos);
                    let mut new_first = second_packet;
                    new_first.len += 1;
                    packets[pos] = new_first;
                    return true;
                }
            }
        }
    }
    save_packet_at_position(undo_stack, packets[pos], pos);
    if pick_random_next_packet_from_top_k(lzma_state, packet_finder, packets, false) {
        return true;
    }
    false
}

fn repair_remaining_packets(
    undo_stack: &mut PacketSlabUndoStack,
    lzma_state: &mut LZMAState,
    enc: &mut dyn EncoderInterface,
    packet_finder: &mut TopKPacketFinder,
    packets: &mut [LZMAPacket],
) {
    let mut count = 0usize;
    let mut rng = rand::thread_rng();
    while lzma_state.position < lzma_state.data.len() {
        count += 1;
        let pos = lzma_state.position;
        let old_packet = packets[pos];
        let mut new_packet = old_packet;

        if new_packet.packet_type == LZMAPacketType::ShortRep
            || new_packet.packet_type == LZMAPacketType::Literal
        {
            // Compute rep0 byte position and check whether it matches the current.
            let dist0 = lzma_state.dists[0] as usize;
            let rep0_valid = pos >= dist0 + 1;
            let same = rep0_valid
                && lzma_state.data[pos] == lzma_state.data[pos - dist0 - 1];
            if same {
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
                packets[pos] = new_packet;
                let best = rng.gen_range(0..4) == 0;
                pick_random_next_packet_from_top_k(lzma_state, packet_finder, packets, best);
                new_packet = packets[pos];
            }
        }

        if !LZMAPacket::cmp(&old_packet, &new_packet) {
            save_packet_at_position(undo_stack, old_packet, pos);
        }
        packets[pos] = new_packet;

        lzma_encode_packet(lzma_state, enc, new_packet);
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
        // Resources are dropped automatically.
    }

    pub fn generate(&mut self, packet_finder: &mut TopKPacketFinder) -> bool {
        let mut lzma_state = self.init_state.clone();
        // Reset perplexity counter for this run.
        self.perplexity = 0;

        let undo_stack = &mut self.undo_stack;
        let perplexity = &mut self.perplexity;
        let packets = self.slab.packets();
        let packet_count = {
            let mut position = 0usize;
            let mut count = 0usize;
            while position < packets.len() {
                count += 1;
                position += packets[position].len as usize;
            }
            count
        };
        if packet_count == 0 {
            return false;
        }
        let mutation_target = rand::thread_rng().gen_range(0..packet_count);

        {
            let mut enc = PerplexityEncoder::new(perplexity);
            encode_to_packet_number(&mut lzma_state, &mut enc, packets, mutation_target);
        }

        if !mutate_next_packet(undo_stack, &lzma_state, packet_finder, packets) {
            return false;
        }
        {
            let packet = packets[lzma_state.position];
            let mut enc = PerplexityEncoder::new(perplexity);
            lzma_encode_packet(&mut lzma_state, &mut enc, packet);
        }

        {
            let mut enc = PerplexityEncoder::new(perplexity);
            repair_remaining_packets(
                undo_stack,
                &mut lzma_state,
                &mut enc,
                packet_finder,
                packets,
            );
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
