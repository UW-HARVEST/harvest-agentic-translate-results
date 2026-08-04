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
        // Drop will clean up everything.
        drop(self);
    }
    fn save_packet_at_position(&mut self, packet: LZMAPacket, position: usize) {
        self.undo_stack.insert(PacketSlabUndo {
            position,
            old_packet: packet,
        });
    }

    fn rand_max_dist(count: usize, num: usize) -> usize {
        // Pick a random number in [0, count) repeatedly and take max, num times.
        let mut rnd = (rand_local() as usize) % count;
        for _ in 1..num {
            let r = (rand_local() as usize) % count;
            if r > rnd {
                rnd = r;
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
        let pos = lzma_state.position;
        packet_finder.find(lzma_state, packets);
        let count = packet_finder.count();
        if count == 0 {
            return false;
        }
        let mut choice = Self::rand_max_dist(count, 8);
        if (rand_local() % 8) == 0 || best {
            choice = count - 1;
        }
        while let Some(p) = packet_finder.pop() {
            if choice == 0 {
                packets[pos] = p;
                return true;
            }
            choice -= 1;
        }
        true
    }

    fn validate_long_rep_packet(lzma_state: &LZMAState, packet: LZMAPacket) -> bool {
        assert!(packet.packet_type == LZMAPacketType::LongRep);
        let pos = lzma_state.position;
        let dist = lzma_state.dists[packet.dist as usize] as usize;
        if pos < dist + 1 {
            return false;
        }
        let rep_start = pos - dist - 1;
        let len = packet.len as usize;
        if rep_start + len > lzma_state.data.len() || pos + len > lzma_state.data.len() {
            return false;
        }
        lzma_state.data[rep_start..rep_start + len] == lzma_state.data[pos..pos + len]
    }

    pub fn generate(&mut self, packet_finder: &mut TopKPacketFinder) -> bool {
        let mut lzma_state = self.init_state.clone();
        let perplexity_ref: *mut u64 = &mut self.perplexity;
        // Initialize perplexity to 0 so cost accumulation begins fresh.
        unsafe { *perplexity_ref = 0 };

        let packet_count = self.slab.count();
        let mutation_target = (rand_local() as usize) % packet_count;

        // Encode up to mutation_target packets
        {
            let packets = self.slab.packets();
            let mut count = 0usize;
            let mut perplexity: u64 = 0;
            while lzma_state.position < lzma_state.data.len() {
                if count == mutation_target {
                    break;
                }
                count += 1;
                let p = packets[lzma_state.position];
                let mut enc = PerplexityEncoder::new(&mut perplexity);
                lzma_encode_packet(&mut lzma_state, &mut enc, p);
            }
            self.perplexity = perplexity;
        }

        // Mutate next packet
        if !self.mutate_next_packet(&lzma_state, packet_finder) {
            return false;
        }

        // Encode the new packet at lzma_state.position
        {
            let packets = self.slab.packets();
            let p = packets[lzma_state.position];
            let mut perplexity = self.perplexity;
            let mut enc = PerplexityEncoder::new(&mut perplexity);
            lzma_encode_packet(&mut lzma_state, &mut enc, p);
            self.perplexity = perplexity;
        }

        self.repair_remaining_packets(&mut lzma_state, packet_finder);
        true
    }

    fn mutate_next_packet(
        &mut self,
        lzma_state: &LZMAState,
        packet_finder: &mut TopKPacketFinder,
    ) -> bool {
        let pos = lzma_state.position;
        let data_size = lzma_state.data.len();
        let packets_len = self.slab.size();
        if pos + 1 < data_size && pos + 1 < packets_len && (rand_local() % 2) == 0 {
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
                    let rep_start: i64 = if second_packet.packet_type == LZMAPacketType::LongRep {
                        pos as i64 - lzma_state.dists[second_packet.dist as usize] as i64
                    } else {
                        pos as i64 - second_packet.dist as i64
                    };
                    if second_packet.len < 273
                        && rep_start > 0
                        && (rep_start as usize) <= data_size
                        && lzma_state.data[pos] == lzma_state.data[(rep_start - 1) as usize]
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
        if Self::pick_random_next_packet_from_top_k(
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
        let mut count = 0usize;
        while lzma_state.position < lzma_state.data.len() {
            count += 1;
            let pos = lzma_state.position;
            let mut packet = self.slab.packets()[pos];
            let old_packet = packet;

            if packet.packet_type == LZMAPacketType::ShortRep
                || packet.packet_type == LZMAPacketType::Literal
            {
                let dist0 = lzma_state.dists[0] as usize;
                if pos >= dist0 + 1
                    && lzma_state.data[pos] == lzma_state.data[pos - dist0 - 1]
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
                while !Self::validate_long_rep_packet(lzma_state, packet) && dist_index < 4 {
                    packet.dist = dist_index;
                    dist_index += 1;
                }
                if !Self::validate_long_rep_packet(lzma_state, packet) {
                    let _best = (rand_local() % 4) == 0;
                    Self::pick_random_next_packet_from_top_k(
                        lzma_state,
                        packet_finder,
                        self.slab.packets(),
                        _best,
                    );
                    packet = self.slab.packets()[pos];
                }
            }

            if !LZMAPacket::cmp(&old_packet, &packet) {
                self.save_packet_at_position(old_packet, pos);
            }
            self.slab.packets()[pos] = packet;

            // Encode using a perplexity encoder
            let mut perplexity = self.perplexity;
            let mut enc = PerplexityEncoder::new(&mut perplexity);
            lzma_encode_packet(lzma_state, &mut enc, packet);
            self.perplexity = perplexity;
        }
    }

    pub fn undo(&mut self) {
        self.undo_stack.apply(&mut self.slab);
    }
    pub fn undo_count(&self) -> usize {
        self.undo_stack.count()
    }
}

fn rand_local() -> u32 {
    use rand::Rng;
    let mut rng = rand::rng();
    rng.random::<u32>()
}
