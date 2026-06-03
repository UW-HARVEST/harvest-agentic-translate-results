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
        // Drop semantics handle resource cleanup; provided for API symmetry
        // with the C version.
    }
    pub fn generate(&mut self, packet_finder: &mut TopKPacketFinder) -> bool {
        let mut lzma_state = self.init_state.clone();

        let packet_count = self.slab.count();
        if packet_count == 0 {
            return false;
        }
        let mut rng = rand::thread_rng();
        let mutation_target = rng.gen_range(0..packet_count);

        // Encode up to mutation_target packets to set up the state.
        {
            let mut perplexity = self.perplexity;
            let mut enc = PerplexityEncoder::new(&mut perplexity);
            let packets = self.slab.packets();
            encode_to_packet_number(
                &mut lzma_state,
                &mut enc as &mut dyn EncoderInterface,
                packets,
                mutation_target,
            );
            // perplexity is dropped at this point with shift_low... but we
            // want to keep the running total.
            drop(enc);
            self.perplexity = perplexity;
        }

        // Mutate the next packet.
        if !self.mutate_next_packet(&lzma_state, packet_finder) {
            return false;
        }
        // Encode the now-mutated packet at the current position.
        {
            let mut perplexity = self.perplexity;
            let mut enc = PerplexityEncoder::new(&mut perplexity);
            let packets = self.slab.packets();
            let packet = packets[lzma_state.position];
            lzma_encode_packet(
                &mut lzma_state,
                &mut enc as &mut dyn EncoderInterface,
                packet,
            );
            drop(enc);
            self.perplexity = perplexity;
        }

        // Repair remaining packets.
        self.repair_remaining_packets(&mut lzma_state, packet_finder);

        true
    }
    pub fn undo(&mut self) {
        self.undo_stack.apply(&mut self.slab);
    }
    pub fn undo_count(&self) -> usize {
        self.undo_stack.count()
    }

    fn save_packet_at_position(&mut self, packet: LZMAPacket, position: usize) {
        self.undo_stack.insert(PacketSlabUndo {
            position,
            old_packet: packet,
        });
    }

    fn mutate_next_packet(
        &mut self,
        lzma_state: &LZMAState,
        packet_finder: &mut TopKPacketFinder,
    ) -> bool {
        let mut rng = rand::thread_rng();
        let position = lzma_state.position;
        let data_size = lzma_state.data.len();
        let packets_view = self.slab.packets();
        let first_packet = packets_view[position];

        if position + 1 < data_size && rng.gen_range(0..2) == 0 {
            let second_packet = packets_view[position + 1];
            if (first_packet.packet_type == LZMAPacketType::LongRep
                || first_packet.packet_type == LZMAPacketType::Match)
                && first_packet.len > 2
            {
                self.save_packet_at_position(first_packet, position);
                self.save_packet_at_position(second_packet, position + 1);
                let packets = self.slab.packets();
                let mut new_second = first_packet;
                new_second.len -= 1;
                packets[position + 1] = new_second;
                packets[position] = LZMAPacket::literal_packet();
                return true;
            } else if first_packet.packet_type == LZMAPacketType::Literal
                || first_packet.packet_type == LZMAPacketType::ShortRep
            {
                if second_packet.packet_type == LZMAPacketType::Match
                    || second_packet.packet_type == LZMAPacketType::LongRep
                {
                    let rep_start = if second_packet.packet_type == LZMAPacketType::LongRep {
                        position
                            .wrapping_sub(lzma_state.dists[second_packet.dist as usize] as usize)
                    } else {
                        position.wrapping_sub(second_packet.dist as usize)
                    };
                    if second_packet.len < 273
                        && rep_start > 0
                        && rep_start <= lzma_state.data.len()
                        && lzma_state.data[position] == lzma_state.data[rep_start - 1]
                    {
                        self.save_packet_at_position(first_packet, position);
                        let packets = self.slab.packets();
                        let mut new_first = second_packet;
                        new_first.len += 1;
                        packets[position] = new_first;
                        return true;
                    }
                }
            }
        }
        self.save_packet_at_position(first_packet, position);
        if pick_random_next_packet_from_top_k(
            lzma_state,
            packet_finder,
            self.slab.packets(),
            false,
        ) {
            return true;
        }
        false
    }

    fn repair_remaining_packets(
        &mut self,
        lzma_state: &mut LZMAState,
        packet_finder: &mut TopKPacketFinder,
    ) {
        let mut rng = rand::thread_rng();
        let mut count: usize = 0;
        while lzma_state.position < lzma_state.data.len() {
            count += 1;
            let position = lzma_state.position;
            let old_packet = self.slab.packets()[position];
            let mut packet = old_packet;

            if packet.packet_type == LZMAPacketType::ShortRep
                || packet.packet_type == LZMAPacketType::Literal
            {
                let rep_pos = position.wrapping_sub(lzma_state.dists[0] as usize + 1);
                if position > lzma_state.dists[0] as usize
                    && lzma_state.data[position] == lzma_state.data[rep_pos]
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
                    self.slab.packets()[position] = packet;
                    let _ = pick_random_next_packet_from_top_k(
                        lzma_state,
                        packet_finder,
                        self.slab.packets(),
                        rng.gen_range(0..4) == 0,
                    );
                    packet = self.slab.packets()[position];
                }
            }

            if !LZMAPacket::cmp(&old_packet, &packet) {
                self.save_packet_at_position(old_packet, position);
            }
            self.slab.packets()[position] = packet;

            // Encode it.
            let mut perplexity = self.perplexity;
            {
                let mut enc = PerplexityEncoder::new(&mut perplexity);
                lzma_encode_packet(
                    lzma_state,
                    &mut enc as &mut dyn EncoderInterface,
                    packet,
                );
            }
            self.perplexity = perplexity;
        }
    }
}

fn encode_to_packet_number(
    lzma_state: &mut LZMAState,
    enc: &mut dyn EncoderInterface,
    packets: &[LZMAPacket],
    target: usize,
) {
    let mut count: usize = 0;
    while lzma_state.position < lzma_state.data.len() {
        if count == target {
            break;
        }
        count += 1;
        let packet = packets[lzma_state.position];
        lzma_encode_packet(lzma_state, enc, packet);
    }
}

fn validate_long_rep_packet(lzma_state: &LZMAState, packet: LZMAPacket) -> bool {
    assert!(packet.packet_type == LZMAPacketType::LongRep);
    let dist = lzma_state.dists[packet.dist as usize] as usize;
    if lzma_state.position == 0 || lzma_state.position <= dist {
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
    &lzma_state.data[rep_start..rep_start + len]
        == &lzma_state.data[lzma_state.position..lzma_state.position + len]
}

fn rand_max_dist(count: usize, num: usize) -> usize {
    let mut rng = rand::thread_rng();
    let mut rnd = rng.gen_range(0..count);
    let mut remaining = num;
    while remaining > 1 {
        let v = rng.gen_range(0..count);
        if v > rnd {
            rnd = v;
        }
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
    let position = lzma_state.position;
    packet_finder.find(lzma_state, packets);
    let count = packet_finder.count();
    if count == 0 {
        return false;
    }
    let mut rng = rand::thread_rng();
    let mut choice = rand_max_dist(count, 8);
    if rng.gen_range(0..8) == 0 || best {
        choice = count - 1;
    }
    while let Some(next) = packet_finder.pop() {
        if choice == 0 {
            packets[position] = next;
            return true;
        }
        choice -= 1;
    }
    true
}
