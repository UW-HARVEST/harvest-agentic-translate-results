use crate::lzma_packet::{LZMAPacket, LZMAPacketType};
use crate::lzma_packet_encoder::lzma_encode_packet;
use crate::lzma_state::LZMAState;
use crate::max_heap::MaxHeap;
use crate::packet_enumerator::PacketEnumerator;
use crate::perplexity_encoder::PerplexityEncoder;
use std::cell::RefCell;

/// A single entry for a Top-K packet candidate.
pub struct TopKEntry {
    pub packet: LZMAPacket,
    pub cost: f32,
}
/// Finds the top-K best packets for encoding.
///
/// The `entries` and `next_packets` fields store candidate packets and the
/// corresponding next packet data, respectively. The `heap` helps maintain the
/// candidates in order, and `packet_enumerator` provides access to the original data.
pub struct TopKPacketFinder<'a> {
    /// The maximum number of entries.
    pub size: usize,
    /// A vector holding the candidate TopK entries.
    pub entries: Vec<TopKEntry>,
    /// A vector holding the next packet values.
    pub next_packets: Vec<LZMAPacket>,
    /// A max-heap structure used for maintaining the order of entries.
    pub heap: Box<MaxHeap>,
    /// A reference to the packet enumerator used to generate candidate packets.
    pub packet_enumerator: &'a PacketEnumerator<'a>,
}

fn sign(x: f32) -> i32 {
    if x < 0.0 {
        -1
    } else if x > 0.0 {
        1
    } else {
        0
    }
}

impl<'a> TopKPacketFinder<'a> {
    pub fn new(size: usize, packet_enumerator: &'a PacketEnumerator<'a>) -> Self {
        // Pre-allocate the entries vector. We initialise with placeholder
        // entries; the buffer's heap allocation will not move as long as we
        // do not push past its capacity.
        let mut entries: Vec<TopKEntry> = Vec::with_capacity(size);
        for _ in 0..size {
            entries.push(TopKEntry {
                packet: LZMAPacket::literal_packet(),
                cost: 0.0,
            });
        }

        // Capture a raw pointer to the entries' backing buffer so the heap
        // comparator can compare entries by cost. This is safe because we
        // never reallocate `entries` (we only assign to existing slots).
        let entries_ptr: *const TopKEntry = entries.as_ptr();
        let comparator = Box::new(move |a: u32, b: u32| -> i32 {
            // SAFETY: the entries buffer outlives the heap because both are
            // owned by `TopKPacketFinder` and the buffer is never reallocated.
            unsafe {
                let entry_a = &*entries_ptr.add(a as usize);
                let entry_b = &*entries_ptr.add(b as usize);
                sign(entry_a.cost - entry_b.cost)
            }
        });

        let heap = Box::new(MaxHeap::new(size, comparator));

        Self {
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

        // Collect candidate packets to avoid borrowing issues with the
        // shared-by-reference packet enumerator iteration callback.
        let collected: RefCell<Vec<LZMAPacket>> = RefCell::new(Vec::new());
        self.packet_enumerator
            .for_each(lzma_state, |_state, packet| {
                collected.borrow_mut().push(packet);
            });
        let candidates = collected.into_inner();

        for packet in candidates {
            // Skip if it is the same as the existing next-packet at this position.
            if LZMAPacket::cmp(&packet, &next_packets[lzma_state.position]) {
                continue;
            }
            // Skip invalid packet types
            if packet.packet_type == LZMAPacketType::Invalid {
                continue;
            }

            let mut new_state = lzma_state.clone();
            let mut perplexity: u64 = 0;
            let start_position = new_state.position;
            {
                let mut enc = PerplexityEncoder::new(&mut perplexity);
                lzma_encode_packet(&mut new_state, &mut enc, packet);
            }

            let length = new_state.position - start_position;
            if length == 0 {
                continue;
            }
            let cost = (perplexity as f32) / (length as f32);
            let entry = TopKEntry { packet, cost };
            self.insert_entry(entry);
        }
    }
    pub fn pop(&mut self) -> Option<LZMAPacket> {
        let max = self.heap.maximum()?;
        let packet = self.entries[max as usize].packet;
        self.heap.remove_maximum();
        Some(packet)
    }
}
