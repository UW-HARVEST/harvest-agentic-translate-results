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
    /// A vector holding the candidate TopK entries.
    pub entries: Vec<TopKEntry>,
    /// A vector holding the next packet values.
    pub next_packets: Vec<LZMAPacket>,
    /// A max-heap structure used for maintaining the order of entries.
    pub heap: Box<MaxHeap>,
    /// A reference to the packet enumerator used to generate candidate packets.
    pub packet_enumerator: &'a PacketEnumerator<'a>,
    /// Shared cost vector used by the comparator to peek at entry costs.
    pub costs: Rc<RefCell<Vec<f32>>>,
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
        let costs: Rc<RefCell<Vec<f32>>> = Rc::new(RefCell::new(Vec::with_capacity(size)));
        let costs_for_cmp = Rc::clone(&costs);
        let comparator = Box::new(move |a: u32, b: u32| -> i32 {
            let costs = costs_for_cmp.borrow();
            sign(costs[a as usize] - costs[b as usize])
        });
        Self {
            size,
            entries: Vec::with_capacity(size),
            next_packets: Vec::new(),
            heap: Box::new(MaxHeap::new(size, comparator)),
            packet_enumerator,
            costs,
        }
    }

    pub fn count(&self) -> usize {
        self.heap.count()
    }

    fn insert_entry(&mut self, entry: TopKEntry) {
        let count = self.heap.count();
        if count < self.size {
            let pos = self.entries.len();
            self.costs.borrow_mut().push(entry.cost);
            self.entries.push(entry);
            self.heap.insert(pos as u32);
            return;
        }
        // Otherwise, replace the maximum if our cost is better (lower).
        let maximum = self.heap.maximum().expect("heap not empty");
        let max_idx = maximum as usize;
        let max_cost = self.entries[max_idx].cost;
        if entry.cost <= max_cost {
            self.costs.borrow_mut()[max_idx] = entry.cost;
            self.entries[max_idx] = entry;
            self.heap.update_maximum();
        }
    }

    pub fn find(&mut self, lzma_state: &LZMAState, next_packets: &mut [LZMAPacket]) {
        // Save next_packets data into the finder so the callback can compare.
        self.next_packets = next_packets.to_vec();
        self.heap.clear();
        self.entries.clear();
        self.costs.borrow_mut().clear();

        // Collect candidates first, then insert them, to avoid borrow conflicts.
        let candidates: RefCell<Vec<(LZMAPacket, f32)>> = RefCell::new(Vec::new());
        let position = lzma_state.position;
        let next_packet_at_pos = self.next_packets[position];

        self.packet_enumerator.for_each(lzma_state, |state, packet| {
            if LZMAPacket::cmp(&packet, &next_packet_at_pos) {
                return;
            }
            let mut new_state = state.clone();
            let mut enc = PerplexityEncoder::new();
            let start_position = new_state.position;
            lzma_encode_packet(&mut new_state, &mut enc, packet);
            let length = new_state.position - start_position;
            if length == 0 {
                return;
            }
            let cost = enc.perplexity as f32 / length as f32;
            candidates.borrow_mut().push((packet, cost));
        });

        for (packet, cost) in candidates.into_inner() {
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
