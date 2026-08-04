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
    pub entries: Box<[TopKEntry]>,
    pub next_packets: Vec<LZMAPacket>,
    pub heap: Box<MaxHeap>,
    pub packet_enumerator: &'a PacketEnumerator<'a>,
}

impl<'a> TopKPacketFinder<'a> {
    pub fn new(size: usize, packet_enumerator: &'a PacketEnumerator<'a>) -> Self {
        let mut entries_vec: Vec<TopKEntry> = Vec::with_capacity(size);
        for _ in 0..size {
            entries_vec.push(TopKEntry {
                packet: LZMAPacket::literal_packet(),
                cost: 0.0,
            });
        }
        let entries: Box<[TopKEntry]> = entries_vec.into_boxed_slice();

        // Get a stable pointer to the boxed slice data
        let entries_data_ptr = entries.as_ptr() as usize;

        let heap = Box::new(MaxHeap::new(
            size,
            Box::new(move |a: u32, b: u32| {
                let entry_a = unsafe { &*(entries_data_ptr as *const TopKEntry).add(a as usize) };
                let entry_b = unsafe { &*(entries_data_ptr as *const TopKEntry).add(b as usize) };
                if entry_a.cost < entry_b.cost {
                    -1
                } else if entry_a.cost > entry_b.cost {
                    1
                } else {
                    0
                }
            }),
        ));

        TopKPacketFinder {
            size,
            entries,
            next_packets: Vec::new(),
            heap,
            packet_enumerator,
        }
    }

    pub fn count(&self) -> usize {
        self.heap.count()
    }

    fn insert_entry(&mut self, entry: TopKEntry) {
        let count = self.heap.count();
        if count < self.size {
            let pos = count;
            self.entries[pos] = entry;
            self.heap.insert(pos as u32);
            return;
        }

        if let Some(maximum) = self.heap.maximum() {
            let max_idx = maximum as usize;
            if entry.cost <= self.entries[max_idx].cost {
                self.entries[max_idx] = entry;
                self.heap.update_maximum();
            }
        }
    }

    pub fn find(&mut self, lzma_state: &LZMAState, next_packets: &mut [LZMAPacket]) {
        self.heap.clear();

        let mut candidates: Vec<(LZMAPacket, f32)> = Vec::new();

        self.packet_enumerator.for_each(lzma_state, |state, packet| {
            if LZMAPacket::cmp(&packet, &next_packets[state.position]) {
                return;
            }

            let mut new_state = state.clone();
            let mut perplexity: u64 = 0;
            let start_position = new_state.position;
            {
                let mut enc = PerplexityEncoder::new(&mut perplexity);
                lzma_encode_packet(&mut new_state, &mut enc, packet);
            }
            let length = new_state.position - start_position;
            let cost = perplexity as f32 / length as f32;
            candidates.push((packet, cost));
        });

        for (packet, cost) in candidates {
            self.insert_entry(TopKEntry { packet, cost });
        }
    }

    pub fn pop(&mut self) -> Option<LZMAPacket> {
        let maximum = self.heap.maximum()?;
        let packet = self.entries[maximum as usize].packet;
        self.heap.remove_maximum();
        Some(packet)
    }
}
