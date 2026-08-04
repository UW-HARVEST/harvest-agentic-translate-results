use crate::lzma_packet::LZMAPacket;
use crate::lzma_packet_encoder::lzma_encode_packet;
use crate::lzma_state::LZMAState;
use crate::max_heap::MaxHeap;
use crate::packet_enumerator::PacketEnumerator;
use crate::perplexity_encoder::PerplexityEncoder;
use std::cell::RefCell;
use std::rc::Rc;

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
    /// A vector holding the candidate TopK entries (shared with the heap comparator).
    pub entries: Rc<RefCell<Vec<TopKEntry>>>,
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
        let entries: Rc<RefCell<Vec<TopKEntry>>> = Rc::new(RefCell::new(Vec::with_capacity(size)));
        let entries_for_cmp = Rc::clone(&entries);
        let comparator: Box<dyn Fn(u32, u32) -> i32> = Box::new(move |a, b| {
            let entries = entries_for_cmp.borrow();
            sign(entries[a as usize].cost - entries[b as usize].cost)
        });
        Self {
            size,
            entries,
            next_packets: Vec::new(),
            heap: Box::new(MaxHeap::new(size, comparator)),
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
            // Push or set the entry at index `pos`.
            {
                let mut entries = self.entries.borrow_mut();
                if entries.len() == pos {
                    entries.push(entry);
                } else {
                    entries[pos] = entry;
                }
            }
            self.heap.insert(pos as u32);
            return;
        }
        // Otherwise, compare with the maximum and replace if cheaper.
        let maximum = self.heap.maximum().expect("expected non-empty heap");
        let max_cost = self.entries.borrow()[maximum as usize].cost;
        if entry.cost <= max_cost {
            self.entries.borrow_mut()[maximum as usize] = entry;
            self.heap.update_maximum();
        }
    }

    pub fn find(&mut self, lzma_state: &LZMAState, next_packets: &mut [LZMAPacket]) {
        // Save next_packets snapshot to compare against.
        self.next_packets = next_packets.to_vec();
        self.heap.clear();

        // Collect candidate packets and their associated cost via the enumerator.
        // The enumerator's callback only borrows the state for the call's
        // duration; we need to clone it before doing any encoding work.
        let mut pending: Vec<(LZMAPacket, u64, usize)> = Vec::new();
        let next_packets_snapshot = self.next_packets.clone();
        self.packet_enumerator.for_each(lzma_state, |state, packet| {
            let pos = state.position;
            if pos < next_packets_snapshot.len()
                && LZMAPacket::cmp(&packet, &next_packets_snapshot[pos])
            {
                return;
            }
            // Encode the packet on a cloned state to determine its perplexity.
            let mut new_state = state.clone();
            let start_position = new_state.position;
            let mut perplexity: u64 = 0;
            {
                let mut enc = PerplexityEncoder::new(&mut perplexity);
                lzma_encode_packet(&mut new_state, &mut enc, packet);
            }
            let length = new_state.position - start_position;
            pending.push((packet, perplexity, length));
        });

        for (packet, perplexity, length) in pending {
            if length == 0 {
                continue;
            }
            let cost = (perplexity / length as u64) as f32;
            let entry = TopKEntry { packet, cost };
            self.insert_entry(entry);
        }
    }

    pub fn pop(&mut self) -> Option<LZMAPacket> {
        let maximum = self.heap.maximum()?;
        let packet = self.entries.borrow()[maximum as usize].packet;
        self.heap.remove_maximum();
        Some(packet)
    }
}
