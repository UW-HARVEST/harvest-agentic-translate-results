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
        // We use RefCell to allow the comparator closure to borrow entries
        // But MaxHeap takes Box<dyn Fn> — we need a way to compare entries.
        // The C code passes `finder` as comparator_data. We'll use a shared entries vec via raw pointer.
        let entries: Vec<TopKEntry> = Vec::with_capacity(size);

        // We can't easily do this with safe Rust and the current MaxHeap API.
        // Instead, we'll create a dummy heap and replace it after.
        let heap = Box::new(MaxHeap::new(size, Box::new(move |_a, _b| 0)));

        let mut finder = TopKPacketFinder {
            size,
            entries,
            next_packets: Vec::new(),
            heap,
            packet_enumerator,
        };

        // Now create the real heap with a comparator that uses a raw pointer to entries
        let entries_ptr = &finder.entries as *const Vec<TopKEntry> as usize;
        finder.heap = Box::new(MaxHeap::new(
            size,
            Box::new(move |a: u32, b: u32| {
                let entries = unsafe { &*(entries_ptr as *const Vec<TopKEntry>) };
                let cost_a = entries[a as usize].cost;
                let cost_b = entries[b as usize].cost;
                if cost_a < cost_b {
                    -1
                } else if cost_a > cost_b {
                    1
                } else {
                    0
                }
            }),
        ));

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
            let max_idx = maximum as usize;
            if entry.cost <= self.entries[max_idx].cost {
                self.entries[max_idx] = entry;
                self.heap.update_maximum();
            }
        }
    }

    pub fn find(&mut self, lzma_state: &LZMAState, next_packets: &mut [LZMAPacket]) {
        // Store a reference to next_packets for the callback
        // We need to use interior mutability pattern here
        self.heap.clear();

        // We collect packets first, then insert them
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
