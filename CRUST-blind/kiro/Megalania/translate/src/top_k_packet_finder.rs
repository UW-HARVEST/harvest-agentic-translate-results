use crate::lzma_packet::LZMAPacket;
use crate::lzma_packet_encoder::lzma_encode_packet;
use crate::lzma_state::LZMAState;
use crate::max_heap::MaxHeap;
use crate::packet_enumerator::PacketEnumerator;
use crate::perplexity_encoder::PerplexityEncoder;

/// A single entry for a Top-K packet candidate.
pub struct TopKEntry {
    pub packet: LZMAPacket,
    pub cost: f32,
}
/// Finds the top-K best packets for encoding.
pub struct TopKPacketFinder<'a> {
    pub size: usize,
    pub entries: Vec<TopKEntry>,
    pub next_packets: Vec<LZMAPacket>,
    pub heap: Box<MaxHeap>,
    pub packet_enumerator: &'a PacketEnumerator<'a>,
}

impl<'a> TopKPacketFinder<'a> {
    pub fn new(size: usize, packet_enumerator: &'a PacketEnumerator<'a>) -> Self {
        let entries: Vec<TopKEntry> = Vec::with_capacity(size);
        let heap = Box::new(MaxHeap::new(size, Box::new(|_a: u32, _b: u32| -> i32 { 0 })));

        let mut finder = TopKPacketFinder {
            size,
            entries,
            next_packets: Vec::new(),
            heap,
            packet_enumerator,
        };

        // Create the real heap with a comparator that uses a raw pointer to entries
        let entries_ptr = &finder.entries as *const Vec<TopKEntry>;
        finder.heap = Box::new(MaxHeap::new(size, Box::new(move |a: u32, b: u32| -> i32 {
            let entries = unsafe { &*entries_ptr };
            let cost_a = entries[a as usize].cost;
            let cost_b = entries[b as usize].cost;
            if cost_a < cost_b { -1 }
            else if cost_a > cost_b { 1 }
            else { 0 }
        })));

        finder
    }
    pub fn count(&self) -> usize {
        self.heap.count()
    }

    fn insert_entry(&mut self, entry: TopKEntry) {
        let count = self.heap.count();
        if count < self.size {
            let pos = count;
            if pos < self.entries.len() {
                self.entries[pos] = entry;
            } else {
                self.entries.push(entry);
            }
            self.heap.insert(pos as u32);
            return;
        }

        if let Some(maximum) = self.heap.maximum() {
            if entry.cost <= self.entries[maximum as usize].cost {
                self.entries[maximum as usize] = entry;
                self.heap.update_maximum();
            }
        }
    }

    pub fn find(&mut self, lzma_state: &LZMAState, next_packets: &mut [LZMAPacket]) {
        self.next_packets = next_packets.to_vec();
        self.entries.clear();
        
        // Rebuild heap with fresh comparator pointing to current entries
        let entries_ptr = &self.entries as *const Vec<TopKEntry>;
        self.heap = Box::new(MaxHeap::new(self.size, Box::new(move |a: u32, b: u32| -> i32 {
            let entries = unsafe { &*entries_ptr };
            let cost_a = entries[a as usize].cost;
            let cost_b = entries[b as usize].cost;
            if cost_a < cost_b { -1 }
            else if cost_a > cost_b { 1 }
            else { 0 }
        })));

        // Collect all candidate packets first
        let mut candidates: Vec<LZMAPacket> = Vec::new();
        self.packet_enumerator.for_each(lzma_state, |_state, packet| {
            candidates.push(packet);
        });

        for packet in candidates {
            if LZMAPacket::cmp(&packet, &self.next_packets[lzma_state.position]) {
                continue;
            }

            let mut new_state = lzma_state.clone();
            let mut perplexity: u64 = 0;
            let start_position = new_state.position;
            {
                let mut enc = PerplexityEncoder { perplexity: &mut perplexity };
                lzma_encode_packet(&mut new_state, &mut enc, packet);
            }

            let length = new_state.position - start_position;
            let entry = TopKEntry { packet, cost: perplexity as f32 / length as f32 };
            self.insert_entry(entry);
        }
    }
    pub fn pop(&mut self) -> Option<LZMAPacket> {
        let maximum = self.heap.maximum()?;
        let packet = self.entries[maximum as usize].packet;
        self.heap.remove_maximum();
        Some(packet)
    }
}
