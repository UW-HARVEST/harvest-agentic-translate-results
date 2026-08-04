use crate::lzma_packet::LZMAPacket;
use crate::lzma_state::LZMAState;
use crate::max_heap::MaxHeap;
use crate::packet_enumerator::PacketEnumerator;
use crate::perplexity_encoder::PerplexityEncoder;
use crate::lzma_packet_encoder::lzma_encode_packet;
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
    /// Sidecar cost cache shared with the heap's comparator.
    costs: Rc<RefCell<Vec<f32>>>,
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
        let costs: Rc<RefCell<Vec<f32>>> = Rc::new(RefCell::new(vec![0.0; size]));
        let costs_for_cmp = Rc::clone(&costs);
        let comparator: Box<dyn Fn(u32, u32) -> i32> = Box::new(move |a: u32, b: u32| {
            let costs = costs_for_cmp.borrow();
            sign(costs[a as usize] - costs[b as usize])
        });
        let heap = Box::new(MaxHeap::new(size, comparator));
        let mut entries = Vec::with_capacity(size);
        for _ in 0..size {
            entries.push(TopKEntry {
                packet: LZMAPacket::literal_packet(),
                cost: 0.0,
            });
        }
        TopKPacketFinder {
            size,
            entries,
            next_packets: Vec::new(),
            heap,
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
            let pos = count;
            self.entries[pos] = entry;
            self.costs.borrow_mut()[pos] = self.entries[pos].cost;
            let _ = self.heap.insert(pos as u32);
            return;
        }

        if let Some(maximum) = self.heap.maximum() {
            let max_idx = maximum as usize;
            if entry.cost <= self.entries[max_idx].cost {
                self.entries[max_idx] = entry;
                self.costs.borrow_mut()[max_idx] = self.entries[max_idx].cost;
                let _ = self.heap.update_maximum();
            }
        }
    }

    pub fn find(&mut self, lzma_state: &LZMAState, next_packets: &mut [LZMAPacket]) {
        // Stash next packets so we can avoid re-considering current choices.
        // To avoid lifetime issues we just copy what we need (the value at
        // each candidate's position). The C version stores a pointer.
        self.next_packets.clear();
        self.next_packets.extend_from_slice(next_packets);
        self.heap.clear();

        // Collect candidate packets to avoid borrow conflicts during enumeration.
        let candidates: RefCell<Vec<(LZMAPacket, f32)>> = RefCell::new(Vec::new());
        let next_packets_ref = &self.next_packets;
        self.packet_enumerator
            .for_each(lzma_state, |state, packet| {
                // Skip if this candidate equals the current next_packet at this position.
                if state.position < next_packets_ref.len() {
                    let cur = next_packets_ref[state.position];
                    if LZMAPacket::cmp(&packet, &cur) {
                        return;
                    }
                }

                let mut new_state = state.clone();
                let start_position = new_state.position;
                let mut perplexity: u64 = 0;
                {
                    let mut enc = PerplexityEncoder::new(&mut perplexity);
                    lzma_encode_packet(&mut new_state, &mut enc, packet);
                }
                let length = new_state.position - start_position;
                if length == 0 {
                    return;
                }
                let cost = (perplexity as f32) / (length as f32);
                candidates.borrow_mut().push((packet, cost));
            });

        for (packet, cost) in candidates.into_inner() {
            self.insert_entry(TopKEntry { packet, cost });
        }
    }

    pub fn pop(&mut self) -> Option<LZMAPacket> {
        let maximum = self.heap.maximum()?;
        let packet = self.entries[maximum as usize].packet;
        let _ = self.heap.remove_maximum();
        Some(packet)
    }
}
