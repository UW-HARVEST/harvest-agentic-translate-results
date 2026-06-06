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
pub struct TopKPacketFinder<'a> {
    pub size: usize,
    pub entries: Vec<TopKEntry>,
    pub next_packets: Vec<LZMAPacket>,
    pub heap: Box<MaxHeap>,
    pub packet_enumerator: &'a PacketEnumerator<'a>,
}

impl<'a> TopKPacketFinder<'a> {
    pub fn new(size: usize, packet_enumerator: &'a PacketEnumerator<'a>) -> Self {
        // Construct a placeholder heap; we'll rebuild a real one (with a closure that
        // can read entry costs) every time `find` is called.
        let comparator: Box<dyn Fn(u32, u32) -> i32> = Box::new(|_a: u32, _b: u32| 0);
        let heap = Box::new(MaxHeap::new(size, comparator));
        let mut entries = Vec::with_capacity(size);
        for _ in 0..size {
            entries.push(TopKEntry {
                packet: LZMAPacket::literal_packet(),
                cost: 0.0,
            });
        }
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

    pub fn find(&mut self, lzma_state: &LZMAState, next_packets: &mut [LZMAPacket]) {
        let size = self.size;
        self.next_packets = next_packets.to_vec();

        // Use a Rc<RefCell<Vec<f32>>> for costs that the heap comparator reads.
        let costs: Rc<RefCell<Vec<f32>>> = Rc::new(RefCell::new(vec![f32::NEG_INFINITY; size]));
        let costs_for_cmp = Rc::clone(&costs);
        let comparator: Box<dyn Fn(u32, u32) -> i32> = Box::new(move |a: u32, b: u32| -> i32 {
            let c = costs_for_cmp.borrow();
            let ca = c[a as usize];
            let cb = c[b as usize];
            let diff = ca - cb;
            if diff < 0.0 {
                -1
            } else if diff > 0.0 {
                1
            } else {
                0
            }
        });
        let mut new_heap = Box::new(MaxHeap::new(size, comparator));

        // Use RefCells so the closure can mutate the heap, costs, entries, and counter.
        let count_cell: RefCell<usize> = RefCell::new(0);
        let entries_cell: RefCell<&mut Vec<TopKEntry>> = RefCell::new(&mut self.entries);
        let heap_ref_cell: RefCell<&mut MaxHeap> = RefCell::new(new_heap.as_mut());
        let costs_for_writer = Rc::clone(&costs);

        let next_packets_local = self.next_packets.clone();

        self.packet_enumerator.for_each(lzma_state, |state, packet| {
            // Skip if packet matches the next_packets[state.position]
            if state.position < next_packets_local.len() {
                if LZMAPacket::cmp(&packet, &next_packets_local[state.position]) {
                    return;
                }
            }
            let mut new_state = state.clone();
            let mut perplexity: u64 = 0;
            let start_position = new_state.position;
            {
                let mut enc = PerplexityEncoder {
                    perplexity: &mut perplexity,
                };
                lzma_encode_packet(&mut new_state, &mut enc, packet);
            }
            let length = new_state.position - start_position;
            if length == 0 {
                return;
            }
            let cost = (perplexity as f32) / (length as f32);

            let mut count_ref = count_cell.borrow_mut();
            let mut entries_ref = entries_cell.borrow_mut();
            let mut heap_ref = heap_ref_cell.borrow_mut();
            let mut costs_ref = costs_for_writer.borrow_mut();

            if *count_ref < size {
                let pos = *count_ref;
                entries_ref[pos].packet = packet;
                entries_ref[pos].cost = cost;
                costs_ref[pos] = cost;
                heap_ref.insert(pos as u32);
                *count_ref += 1;
            } else if let Some(maximum) = heap_ref.maximum() {
                let max_idx = maximum as usize;
                if cost <= entries_ref[max_idx].cost {
                    entries_ref[max_idx].packet = packet;
                    entries_ref[max_idx].cost = cost;
                    costs_ref[max_idx] = cost;
                    heap_ref.update_maximum();
                }
            }
        });

        self.heap = new_heap;
    }

    pub fn pop(&mut self) -> Option<LZMAPacket> {
        let max = self.heap.maximum()?;
        let packet = self.entries[max as usize].packet;
        self.heap.remove_maximum();
        Some(packet)
    }
}
