use crate::lzma_packet::{LZMAPacket, LZMAPacketType};
use crate::lzma_packet_encoder::lzma_encode_packet;
use crate::lzma_state::LZMAState;
use crate::packet_slab::PacketSlab;
use crate::packet_slab_undo_stack::{PacketSlabUndo, PacketSlabUndoStack};
use crate::perplexity_encoder::PerplexityEncoder;
use crate::top_k_packet_finder::TopKPacketFinder;
use rand::Rng;
use std::cell::RefCell;
use std::rc::Rc;

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
        // Drop handles cleanup.
    }

    pub fn generate(&mut self, packet_finder: &mut TopKPacketFinder) -> bool {
        let packet_count = self.slab.count();
        if packet_count == 0 {
            return false;
        }
        let mut rng = rand::rng();
        let mutation_target = rng.random_range(0..packet_count);

        let perplexity_rc: Rc<RefCell<u64>> = Rc::new(RefCell::new(0u64));

        let mut lzma_state = self.init_state.clone();
        let mut enc = PerplexityEncoder::new(Rc::clone(&perplexity_rc));

        // encode_to_packet_number
        {
            let packets = self.slab.packets();
            let mut count = 0usize;
            while lzma_state.position < lzma_state.data.len() {
                if count == mutation_target {
                    break;
                }
                count += 1;
                let p = packets[lzma_state.position];
                lzma_encode_packet(&mut lzma_state, &mut enc, p);
            }
        }

        // mutate_next_packet
        if !self.mutate_next_packet(&lzma_state, packet_finder) {
            self.perplexity = *perplexity_rc.borrow();
            return false;
        }
        // encode the (now mutated) next packet
        {
            let packets = self.slab.packets();
            let p = packets[lzma_state.position];
            lzma_encode_packet(&mut lzma_state, &mut enc, p);
        }

        // repair_remaining_packets
        self.repair_remaining_packets(&mut lzma_state, &mut enc, packet_finder);

        self.perplexity = *perplexity_rc.borrow();
        true
    }

    fn save_packet(&mut self, packet: LZMAPacket, position: usize) {
        let undo = PacketSlabUndo {
            position,
            old_packet: packet,
        };
        self.undo_stack.insert(undo);
    }

    fn pick_random_next_packet_from_top_k(
        &mut self,
        lzma_state: &LZMAState,
        packet_finder: &mut TopKPacketFinder,
        best: bool,
    ) -> bool {
        let position = lzma_state.position;
        let packets = self.slab.packets();
        packet_finder.find(lzma_state, packets);
        let count = packet_finder.count();
        if count == 0 {
            return false;
        }
        let mut rng = rand::rng();
        let mut choice = rand_max_dist(count, 8, &mut rng);
        if rng.random_range(0..8) == 0 || best {
            choice = count - 1;
        }
        let mut chosen: Option<LZMAPacket> = None;
        while let Some(p) = packet_finder.pop() {
            if choice == 0 {
                chosen = Some(p);
                break;
            } else {
                choice -= 1;
            }
        }
        if let Some(p) = chosen {
            self.slab.packets()[position] = p;
        }
        true
    }

    fn mutate_next_packet(
        &mut self,
        lzma_state: &LZMAState,
        packet_finder: &mut TopKPacketFinder,
    ) -> bool {
        let pos = lzma_state.position;
        let data_size = lzma_state.data.len();
        let mut rng = rand::rng();

        if pos + 1 < data_size && rng.random_range(0..2) == 0 {
            let first_packet = self.slab.packets()[pos];
            let second_packet = self.slab.packets()[pos + 1];
            if (matches!(first_packet.packet_type, LZMAPacketType::LongRep)
                || matches!(first_packet.packet_type, LZMAPacketType::Match))
                && first_packet.len > 2
            {
                self.save_packet(first_packet, pos);
                self.save_packet(second_packet, pos + 1);
                let mut new_second = first_packet;
                new_second.len -= 1;
                self.slab.packets()[pos + 1] = new_second;
                self.slab.packets()[pos] = LZMAPacket::literal_packet();
                return true;
            } else if matches!(first_packet.packet_type, LZMAPacketType::Literal)
                || matches!(first_packet.packet_type, LZMAPacketType::ShortRep)
            {
                if matches!(second_packet.packet_type, LZMAPacketType::Match)
                    || matches!(second_packet.packet_type, LZMAPacketType::LongRep)
                {
                    let mut rep_start = pos as i64 - second_packet.dist as i64;
                    if matches!(second_packet.packet_type, LZMAPacketType::LongRep) {
                        rep_start =
                            pos as i64 - lzma_state.dists[second_packet.dist as usize] as i64;
                    }
                    if second_packet.len < 273
                        && rep_start > 0
                        && lzma_state.data[pos] == lzma_state.data[(rep_start - 1) as usize]
                    {
                        self.save_packet(first_packet, pos);
                        let mut new_first = second_packet;
                        new_first.len += 1;
                        self.slab.packets()[pos] = new_first;
                        return true;
                    }
                }
            }
        }
        let cur = self.slab.packets()[pos];
        self.save_packet(cur, pos);
        if self.pick_random_next_packet_from_top_k(lzma_state, packet_finder, false) {
            return true;
        }
        false
    }

    fn validate_long_rep_packet(lzma_state: &LZMAState, packet: LZMAPacket) -> bool {
        assert!(matches!(packet.packet_type, LZMAPacketType::LongRep));
        let dist = lzma_state.dists[packet.dist as usize] as usize;
        if lzma_state.position < dist + 1 {
            return false;
        }
        let rep_start = lzma_state.position - dist - 1;
        let len = packet.len as usize;
        if rep_start + len > lzma_state.data.len() || lzma_state.position + len > lzma_state.data.len() {
            return false;
        }
        lzma_state.data[rep_start..rep_start + len]
            == lzma_state.data[lzma_state.position..lzma_state.position + len]
    }

    fn repair_remaining_packets(
        &mut self,
        lzma_state: &mut LZMAState,
        enc: &mut PerplexityEncoder,
        packet_finder: &mut TopKPacketFinder,
    ) {
        let mut count: usize = 0;
        let mut rng = rand::rng();
        while lzma_state.position < lzma_state.data.len() {
            count += 1;
            let pos = lzma_state.position;
            let old_packet = self.slab.packets()[pos];
            let mut packet = old_packet;

            if matches!(packet.packet_type, LZMAPacketType::ShortRep)
                || matches!(packet.packet_type, LZMAPacketType::Literal)
            {
                let dist0 = lzma_state.dists[0] as usize;
                if pos > dist0
                    && lzma_state.data[pos] == lzma_state.data[pos - dist0 - 1]
                {
                    if count < 4 {
                        packet = LZMAPacket::short_rep_packet();
                    }
                } else {
                    packet = LZMAPacket::literal_packet();
                }
            }
            if matches!(packet.packet_type, LZMAPacketType::LongRep) {
                let mut dist_index = 0u32;
                while !Self::validate_long_rep_packet(lzma_state, packet) && dist_index < 4 {
                    packet.dist = dist_index;
                    dist_index += 1;
                }
                if !Self::validate_long_rep_packet(lzma_state, packet) {
                    self.slab.packets()[pos] = packet;
                    let best = rng.random_range(0..4) == 0;
                    self.pick_random_next_packet_from_top_k(lzma_state, packet_finder, best);
                    packet = self.slab.packets()[pos];
                }
            }

            if !LZMAPacket::cmp(&old_packet, &packet) {
                self.save_packet(old_packet, pos);
            }
            self.slab.packets()[pos] = packet;

            lzma_encode_packet(lzma_state, enc, packet);
        }
    }

    pub fn undo(&mut self) {
        self.undo_stack.apply(&mut self.slab);
    }
    pub fn undo_count(&self) -> usize {
        self.undo_stack.count()
    }
}

fn rand_max_dist(count: usize, num: usize, rng: &mut impl Rng) -> usize {
    let mut rnd = rng.random_range(0..count);
    let mut n = num;
    while n > 1 {
        n -= 1;
        let r = rng.random_range(0..count);
        if r > rnd {
            rnd = r;
        }
    }
    rnd
}
