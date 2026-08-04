use crate::encoder_interface::EncoderInterface;
use crate::lzma_packet::LZMAPacket;
use crate::lzma_packet_encoder::lzma_encode_packet;
use crate::lzma_state::LZMAState;
use crate::max_heap::MaxHeap;
use crate::packet_enumerator::PacketEnumerator;
use crate::perplexity_encoder::PerplexityEncoder;
use crate::probability::Prob;
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
    /// A vector holding the candidate TopK entries (shared with the heap comparator).
    pub entries: Rc<RefCell<Vec<TopKEntry>>>,
    /// A vector holding the next packet values.
    pub next_packets: Vec<LZMAPacket>,
    /// A max-heap structure used for maintaining the order of entries.
    pub heap: Box<MaxHeap>,
    /// A reference to the packet enumerator used to generate candidate packets.
    pub packet_enumerator: &'a PacketEnumerator<'a>,
}
impl<'a> TopKPacketFinder<'a> {
    pub fn new(size: usize, packet_enumerator: &'a PacketEnumerator<'a>) -> Self {
        let entries: Rc<RefCell<Vec<TopKEntry>>> = Rc::new(RefCell::new(Vec::with_capacity(size)));
        let entries_for_cmp = Rc::clone(&entries);
        let comparator = Box::new(move |a: u32, b: u32| {
            let entries = entries_for_cmp.borrow();
            let cost_a = entries[a as usize].cost;
            let cost_b = entries[b as usize].cost;
            if cost_a < cost_b {
                -1
            } else if cost_a > cost_b {
                1
            } else {
                0
            }
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
            let pos = count as u32;
            self.entries.borrow_mut().push(entry);
            self.heap.insert(pos);
            return;
        }

        if let Some(maximum) = self.heap.maximum() {
            let max_cost = self.entries.borrow()[maximum as usize].cost;
            if entry.cost <= max_cost {
                self.entries.borrow_mut()[maximum as usize] = entry;
                self.heap.update_maximum();
            }
        }
    }
    pub fn find(&mut self, lzma_state: &LZMAState, next_packets: &mut [LZMAPacket]) {
        // Make a local copy of next_packets to mirror the C semantics
        // (which stored a pointer; we keep an owned snapshot for safety).
        self.next_packets = next_packets.to_vec();
        self.heap.clear();
        self.entries.borrow_mut().clear();

        // We need to inspect the next_packets and collect (packet, cost)
        // candidates into a temporary buffer, then insert them. This avoids
        // borrowing self mutably inside the closure.
        let mut candidates: Vec<TopKEntry> = Vec::new();
        let next_packets_snapshot = self.next_packets.clone();
        self.packet_enumerator
            .for_each(lzma_state, |state, packet| {
                if LZMAPacket::cmp(&packet, &next_packets_snapshot[state.position]) {
                    return;
                }
                let mut new_state = state.clone();
                let mut perplexity: u64 = 0;
                let start_position = new_state.position;
                {
                    let mut enc = PerplexityEncoder::new(&mut perplexity);
                    lzma_encode_packet(&mut new_state, &mut enc as &mut dyn EncoderInterface, packet);
                }
                let length = new_state.position - start_position;
                let cost = if length == 0 {
                    f32::INFINITY
                } else {
                    (perplexity as f32) / (length as f32)
                };
                candidates.push(TopKEntry { packet, cost });
            });

        for entry in candidates {
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

// Suppress unused warning for Prob (re-exported via probability module).
#[allow(dead_code)]
fn _prob_ref(_p: Prob) {}
