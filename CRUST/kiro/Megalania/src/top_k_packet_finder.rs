use crate::lzma_packet::LZMAPacket;
use crate::lzma_packet_encoder::lzma_encode_packet;
use crate::lzma_state::LZMAState;
use crate::max_heap::MaxHeap;
use crate::packet_enumerator::PacketEnumerator;
use crate::perplexity_encoder::PerplexityEncoder;
use std::cell::RefCell;
use std::rc::Rc;

pub struct TopKEntry {
    pub packet: LZMAPacket,
    pub cost: f32,
}

pub struct TopKPacketFinder<'a> {
    pub size: usize,
    pub entries: Rc<RefCell<Vec<TopKEntry>>>,
    pub next_packets: Vec<LZMAPacket>,
    pub heap: Box<MaxHeap>,
    pub packet_enumerator: &'a PacketEnumerator<'a>,
}

impl<'a> TopKPacketFinder<'a> {
    pub fn new(size: usize, packet_enumerator: &PacketEnumerator) -> Self {
        let pe: &'a PacketEnumerator<'a> = unsafe { std::mem::transmute(packet_enumerator) };

        let entries = Rc::new(RefCell::new(
            (0..size).map(|_| TopKEntry { packet: LZMAPacket::literal_packet(), cost: 0.0 }).collect::<Vec<_>>()
        ));

        let entries_for_cmp = Rc::clone(&entries);
        let heap = Box::new(MaxHeap::new(size, Box::new(move |a: u32, b: u32| -> i32 {
            let e = entries_for_cmp.borrow();
            let ca = e[a as usize].cost;
            let cb = e[b as usize].cost;
            if ca < cb { -1 } else if ca > cb { 1 } else { 0 }
        })));

        TopKPacketFinder {
            size,
            entries,
            next_packets: Vec::new(),
            heap,
            packet_enumerator: pe,
        }
    }

    pub fn count(&self) -> usize {
        self.heap.count()
    }

    fn insert_entry(&mut self, entry: TopKEntry) {
        let count = self.heap.count();
        if count < self.size {
            let pos = count;
            self.entries.borrow_mut()[pos] = entry;
            self.heap.insert(pos as u32);
        } else if let Some(maximum) = self.heap.maximum() {
            let max_idx = maximum as usize;
            if entry.cost <= self.entries.borrow()[max_idx].cost {
                self.entries.borrow_mut()[max_idx] = entry;
                self.heap.update_maximum();
            }
        }
    }

    pub fn find(&mut self, lzma_state: &LZMAState, next_packets: &mut [LZMAPacket]) {
        self.heap.clear();

        let mut candidates: Vec<LZMAPacket> = Vec::new();
        self.packet_enumerator.for_each(lzma_state, |_state, packet| {
            candidates.push(packet);
        });

        for packet in candidates {
            if LZMAPacket::cmp(&packet, &next_packets[lzma_state.position]) {
                continue;
            }

            let mut new_state = lzma_state.clone();
            let mut perplexity: u64 = 0;
            let start_position = new_state.position;
            {
                let mut enc = PerplexityEncoder { perplexity: &mut perplexity };
                lzma_encode_packet(&mut new_state, &mut enc, packet);
            }
            let length = new_state.position - start_position;
            let entry = TopKEntry { packet, cost: perplexity as f32 / length as f32 };
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
