use crate::encoder_interface::EncoderInterface;
use crate::lzma_packet::LZMAPacket;
use crate::lzma_packet_encoder::lzma_encode_packet;
use crate::lzma_state::LZMAState;
use crate::max_heap::MaxHeap;
use crate::packet_enumerator::PacketEnumerator;
use crate::probability::Prob;
use std::cell::RefCell;

/// A single entry for a Top-K packet candidate.
pub struct TopKEntry {
    pub packet: LZMAPacket,
    pub cost: f32,
}

// Thread-local snapshot of the current finder's entry costs, used by the
// MaxHeap's comparator (which can only see two `u32` indices). Updated by
// the finder before any heap operation.
thread_local! {
    static FINDER_COSTS: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
}

fn set_finder_costs(entries: &[TopKEntry]) {
    FINDER_COSTS.with(|costs| {
        let mut costs = costs.borrow_mut();
        costs.clear();
        for e in entries {
            costs.push(e.cost);
        }
    });
}

fn update_finder_cost(index: usize, cost: f32) {
    FINDER_COSTS.with(|costs| {
        let mut costs = costs.borrow_mut();
        if costs.len() <= index {
            costs.resize(index + 1, 0.0);
        }
        costs[index] = cost;
    });
}

fn cost_comparator(a: u32, b: u32) -> i32 {
    FINDER_COSTS.with(|costs| {
        let costs = costs.borrow();
        let ca = costs[a as usize];
        let cb = costs[b as usize];
        if ca < cb {
            -1
        } else if ca > cb {
            1
        } else {
            0
        }
    })
}

/// A simple perplexity-tracking encoder used internally by the finder.
struct CostEncoder<'a> {
    perplexity: &'a mut u64,
}

impl<'a> EncoderInterface for CostEncoder<'a> {
    fn encode_bit(&mut self, bit: bool, prob: Prob) {
        let idx = if bit {
            (2048u32 - prob as u32) as usize
        } else {
            prob as usize
        };
        *self.perplexity += crate::perplexity_table::LOG2_LOOKUP[idx];
    }

    fn encode_direct_bits(&mut self, _bits: u32, num_bits: u32) {
        *self.perplexity += (num_bits as u64) << 11;
    }
}

/// Finds the top-K best packets for encoding.
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
impl<'a> TopKPacketFinder<'a> {
    pub fn new(size: usize, packet_enumerator: &'a PacketEnumerator<'a>) -> Self {
        let comparator: Box<dyn Fn(u32, u32) -> i32> = Box::new(cost_comparator);
        let heap = Box::new(MaxHeap::new(size, comparator));
        Self {
            size,
            entries: Vec::with_capacity(size),
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
            // There is room: append and insert into heap.
            let pos = self.entries.len();
            update_finder_cost(pos, entry.cost);
            self.entries.push(entry);
            self.heap.insert(pos as u32);
            return;
        }
        // Otherwise, compare with the current maximum (worst entry).
        let maximum = self.heap.maximum().expect("heap is full but has no max");
        let max_cost = self.entries[maximum as usize].cost;
        if entry.cost <= max_cost {
            update_finder_cost(maximum as usize, entry.cost);
            self.entries[maximum as usize] = entry;
            self.heap.update_maximum();
        }
    }

    pub fn find(&mut self, lzma_state: &LZMAState, next_packets: &mut [LZMAPacket]) {
        // Snapshot next_packets so the callback can compare against it without
        // borrowing conflicts.
        self.next_packets.clear();
        self.next_packets.extend_from_slice(next_packets);
        // Reset entries and the heap.
        self.entries.clear();
        self.heap.clear();
        set_finder_costs(&[]);

        // Collect candidates from the packet enumerator first. We can't borrow
        // `self` mutably from inside the callback (it takes `Fn`), so we use
        // a RefCell to accumulate and then process.
        let candidates_cell: RefCell<Vec<LZMAPacket>> = RefCell::new(Vec::new());
        // Track position to filter dupes against next_packets.
        let pos = lzma_state.position;
        let next_packet_at_pos = if pos < self.next_packets.len() {
            Some(self.next_packets[pos])
        } else {
            None
        };

        self.packet_enumerator
            .for_each(lzma_state, |_state, packet| {
                if let Some(np) = next_packet_at_pos {
                    if LZMAPacket::cmp(&packet, &np) {
                        return;
                    }
                }
                candidates_cell.borrow_mut().push(packet);
            });
        let candidates = candidates_cell.into_inner();

        // For each candidate, simulate encoding and compute cost.
        for packet in candidates {
            let mut new_state = lzma_state.clone();
            let mut perplexity: u64 = 0;
            let start_position = new_state.position;
            {
                let mut enc = CostEncoder {
                    perplexity: &mut perplexity,
                };
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
        let maximum = self.heap.maximum()?;
        let packet = self.entries[maximum as usize].packet;
        self.heap.remove_maximum();
        Some(packet)
    }
}
