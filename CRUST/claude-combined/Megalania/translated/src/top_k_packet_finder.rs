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
        // We use a parallel Rc<RefCell<Vec<f32>>> of costs, captured by the heap
        // comparator. We keep `costs` in sync with `entries[i].cost`.
        let costs: Rc<RefCell<Vec<f32>>> = Rc::new(RefCell::new(Vec::with_capacity(size)));
        let costs_for_cmp = Rc::clone(&costs);
        let comparator: Box<dyn Fn(u32, u32) -> i32> =
            Box::new(move |a: u32, b: u32| -> i32 {
                let costs = costs_for_cmp.borrow();
                let ca = costs[a as usize];
                let cb = costs[b as usize];
                sign(ca - cb)
            });
        let heap = Box::new(MaxHeap::new(size, comparator));

        // Stash a clone of `costs` in a thread-local-ish way: actually we cannot
        // store it in the struct directly. We capture it again into a separate
        // `extra_costs` location accessible via methods on `Self` through a
        // hack: store inside the closure. To access from methods we use a
        // simple field workaround—store the Rc inside the entries' first slot
        // implicitly. Instead, re-clone the same `costs` Rc into the entries
        // by leaking it as static? No—we just keep our own copy locally and
        // pass it along through self by reconstructing the costs Rc via the
        // comparator capture... which we can't extract.
        //
        // The simpler approach: maintain a side `Rc<RefCell<Vec<f32>>>` that
        // we keep alive through a "leaked" copy... Actually we can simply
        // carry it via `unsafe` raw pointer is overkill here. Let's just
        // recompute costs each time by reading entries when inserting: we
        // mutate `costs` ourselves inside `find` since we have `costs` here
        // by closure. Actually we can store another clone in a Box leaked to
        // a 'static lifetime. Better: store `costs` as a thread-local. To
        // keep things clean, we use a global registry tied to self pointer.
        //
        // Pragmatic choice: leak `costs` to 'static and stash via a private
        // hashmap keyed by self pointer. To avoid that, we use a much
        // simpler design where `find` rebuilds costs locally and we don't
        // depend on heap ordering being perfectly consistent across calls.
        // This works because the heap is cleared at the start of `find`.

        Self {
            size,
            entries: Vec::with_capacity(size),
            next_packets: Vec::new(),
            heap,
            packet_enumerator,
            // costs Rc is kept alive only through the heap's closure. We
            // also re-create a parallel costs vec inside `find` and update
            // it through a fresh closure if needed. For simplicity in this
            // port we re-implement the heap behavior manually inside our
            // own methods using direct sorting of `entries`.
        }
    }

    pub fn count(&self) -> usize {
        self.heap.count()
    }

    /// Insert an entry into the top-K via direct manipulation. Because we
    /// cannot share state between the heap's comparator and our methods
    /// without unsafe code or extra fields, we implement top-K logic
    /// manually here.
    fn insert_entry(&mut self, entry: TopKEntry) {
        let count = self.heap.count();
        if count < self.size {
            // Push entry, push corresponding index into heap.
            // Sort entries so the highest-cost is at index 0 (heap "top").
            self.entries.push(entry);
            // We need indices in heap to follow ordering by cost descending.
            // Rebuild heap from scratch each time.
            self.rebuild_heap();
            return;
        }
        // Find current maximum (highest cost) index and replace if entry.cost <=
        let max_idx = self.heap.maximum().unwrap() as usize;
        if entry.cost <= self.entries[max_idx].cost {
            self.entries[max_idx] = entry;
            self.rebuild_heap();
        }
    }

    fn rebuild_heap(&mut self) {
        // Recreate heap with current entries' cost ordering.
        // We use a fresh comparator capturing a copy of current costs.
        let costs_snapshot: Vec<f32> = self.entries.iter().map(|e| e.cost).collect();
        let costs_rc: Rc<RefCell<Vec<f32>>> = Rc::new(RefCell::new(costs_snapshot));
        let costs_for_cmp = Rc::clone(&costs_rc);
        let comparator: Box<dyn Fn(u32, u32) -> i32> = Box::new(move |a: u32, b: u32| -> i32 {
            let costs = costs_for_cmp.borrow();
            sign(costs[a as usize] - costs[b as usize])
        });
        let mut new_heap = Box::new(MaxHeap::new(self.size, comparator));
        for i in 0..self.entries.len() {
            new_heap.insert(i as u32);
        }
        self.heap = new_heap;
    }

    pub fn find(&mut self, lzma_state: &LZMAState, next_packets: &mut [LZMAPacket]) {
        // Clone next_packets into our own owned vec.
        self.next_packets = next_packets.to_vec();
        self.entries.clear();
        self.heap.clear();

        // Collect candidates by running the packet enumerator.
        let mut candidates: Vec<LZMAPacket> = Vec::new();
        self.packet_enumerator.for_each(lzma_state, |_state, packet| {
            candidates.push(packet);
        });

        // For each candidate, compute cost via simulation and (maybe) insert.
        for packet in candidates {
            let pos = lzma_state.position;
            if pos < self.next_packets.len()
                && LZMAPacket::cmp(&packet, &self.next_packets[pos])
            {
                continue;
            }
            // Simulate: clone state, encode packet, measure perplexity / length.
            let mut new_state = lzma_state.clone();
            let perplexity_rc: Rc<RefCell<u64>> = Rc::new(RefCell::new(0u64));
            let mut enc = PerplexityEncoder::new(Rc::clone(&perplexity_rc));
            let start_position = new_state.position;
            lzma_encode_packet(&mut new_state, &mut enc, packet);
            let length = new_state.position - start_position;
            if length == 0 {
                continue;
            }
            let perplexity = *perplexity_rc.borrow();
            let cost = (perplexity as f32) / (length as f32);
            let entry = TopKEntry { packet, cost };
            self.insert_entry(entry);
        }
    }

    pub fn pop(&mut self) -> Option<LZMAPacket> {
        let max_idx = self.heap.maximum()?;
        let packet = self.entries[max_idx as usize].packet;
        self.heap.remove_maximum();
        Some(packet)
    }
}
