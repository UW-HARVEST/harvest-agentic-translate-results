use crate::lzma_packet::LZMAPacket;
use crate::lzma_packet_encoder::lzma_encode_packet;
use crate::lzma_state::LZMAState;
use crate::max_heap::MaxHeap;
use crate::packet_enumerator::PacketEnumerator;
use crate::perplexity_encoder::PerplexityEncoder;
use std::cell::RefCell;
use std::rc::Rc;

/// A single entry for a Top-K packet candidate.
#[derive(Clone, Copy)]
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
    /// Shared backing data for comparator
    entries_shared: Rc<RefCell<Vec<TopKEntry>>>,
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
        let initial_entries = vec![
            TopKEntry {
                packet: LZMAPacket::literal_packet(),
                cost: 0.0
            };
            size
        ];
        let entries_shared = Rc::new(RefCell::new(initial_entries.clone()));
        let comparator_shared = entries_shared.clone();
        let comparator = Box::new(move |a: u32, b: u32| -> i32 {
            let entries = comparator_shared.borrow();
            let a_cost = entries[a as usize].cost;
            let b_cost = entries[b as usize].cost;
            sign(a_cost - b_cost)
        });
        let heap = Box::new(MaxHeap::new(size, comparator));
        Self {
            size,
            entries: initial_entries,
            next_packets: Vec::new(),
            heap,
            packet_enumerator,
            entries_shared,
        }
    }
    pub fn count(&self) -> usize {
        self.heap.count()
    }

    fn insert_entry(&mut self, entry: TopKEntry) {
        let count = self.count();
        if count < self.size {
            self.entries[count] = entry;
            // Update shared
            self.entries_shared.borrow_mut()[count] = entry;
            self.heap.insert(count as u32);
            return;
        }
        let max = self.heap.maximum().expect("heap full but no max");
        if entry.cost <= self.entries[max as usize].cost {
            self.entries[max as usize] = entry;
            self.entries_shared.borrow_mut()[max as usize] = entry;
            self.heap.update_maximum();
        }
    }

    pub fn find(&mut self, lzma_state: &LZMAState, next_packets: &mut [LZMAPacket]) {
        // Replace next_packets reference logic
        self.next_packets = next_packets.to_vec();
        self.heap.clear();

        // We need to enumerate candidates and compute cost for each.
        // Collect candidates first to avoid borrow conflicts.
        // Store only the packet and the position used to look up next_packets.
        let mut candidates: Vec<(LZMAPacket, usize)> = Vec::new();
        let next_packets_snapshot = self.next_packets.clone();
        let lzma_state_clone = lzma_state.clone();
        self.packet_enumerator.for_each(&lzma_state_clone, |state, packet| {
            let pos = state.position;
            if pos < next_packets_snapshot.len()
                && LZMAPacket::cmp(&packet, &next_packets_snapshot[pos])
            {
                return;
            }
            candidates.push((packet, pos));
        });

        for (packet, _pos) in candidates {
            let mut new_state = lzma_state.clone();
            let start_position = new_state.position;
            let mut perplexity: u64 = 0;
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
