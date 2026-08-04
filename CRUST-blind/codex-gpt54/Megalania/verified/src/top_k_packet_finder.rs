use crate::lzma_packet::LZMAPacket;
use crate::lzma_state::LZMAState;
use crate::max_heap::MaxHeap;
use crate::packet_enumerator::PacketEnumerator;
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Mutex, OnceLock};
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
impl<'a> TopKPacketFinder<'a> {
    pub fn new(size: usize, packet_enumerator: &PacketEnumerator) -> Self {
        fn registry() -> &'static Mutex<std::collections::HashMap<usize, Vec<f32>>> {
            static REGISTRY: OnceLock<Mutex<std::collections::HashMap<usize, Vec<f32>>>> =
                OnceLock::new();
            REGISTRY.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
        }

        let heap_key = Rc::new(Cell::new(0usize));
        let heap_key_for_cmp = heap_key.clone();
        let heap = Box::new(MaxHeap::new(
            size,
            Box::new(move |a, b| {
                let key = heap_key_for_cmp.get();
                if key == 0 {
                    return 0;
                }
                let guard = registry().lock().expect("top-k cost registry poisoned");
                let Some(costs) = guard.get(&key) else {
                    return 0;
                };
                let a_cost = costs[a as usize];
                let b_cost = costs[b as usize];
                if a_cost < b_cost {
                    -1
                } else if a_cost > b_cost {
                    1
                } else {
                    0
                }
            }),
        ));
        let key = (&*heap as *const MaxHeap) as usize;
        registry()
            .lock()
            .expect("top-k cost registry poisoned")
            .insert(key, vec![0.0; size]);
        heap_key.set(key);

        // SAFETY: the returned finder stores the same borrowed packet enumerator the caller passed in.
        let packet_enumerator =
            unsafe { std::mem::transmute::<&PacketEnumerator<'_>, &'a PacketEnumerator<'a>>(packet_enumerator) };

        Self {
            size,
            entries: (0..size)
                .map(|_| TopKEntry {
                    packet: LZMAPacket::literal_packet(),
                    cost: 0.0,
                })
                .collect(),
            next_packets: Vec::new(),
            heap,
            packet_enumerator,
        }
    }
    pub fn count(&self) -> usize {
        self.heap.count()
    }
    pub fn find(&mut self, lzma_state: &LZMAState, next_packets: &mut [LZMAPacket]) {
        fn registry() -> &'static Mutex<std::collections::HashMap<usize, Vec<f32>>> {
            static REGISTRY: OnceLock<Mutex<std::collections::HashMap<usize, Vec<f32>>>> =
                OnceLock::new();
            REGISTRY.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
        }

        struct DummyEncoder {
            _tag: u8,
        }
        impl crate::encoder_interface::EncoderInterface for DummyEncoder {
            fn encode_bit(&mut self, _bit: bool, _prob: crate::probability::Prob) {}
            fn encode_direct_bits(&mut self, _bits: u32, _num_bits: u32) {}
        }

        let heap_key = (&*self.heap as *const MaxHeap) as usize;
        self.next_packets.clear();
        self.next_packets.extend_from_slice(next_packets);
        self.heap.clear();

        let candidates = RefCell::new(Vec::new());
        self.packet_enumerator.for_each(lzma_state, |_state, packet| {
            candidates.borrow_mut().push(packet);
        });

        for packet in candidates.into_inner() {
            if self
                .next_packets
                .get(lzma_state.position)
                .is_some_and(|next| LZMAPacket::cmp(&packet, next))
            {
                continue;
            }

            let mut new_state = lzma_state.clone();
            let mut enc = DummyEncoder { _tag: 0 };
            let mut perplexity = 0u64;
            let start_position = new_state.position;
            crate::perplexity_encoder::perplexity_encoder_new(&mut enc, &mut perplexity);
            crate::lzma_packet_encoder::lzma_encode_packet(&mut new_state, &mut enc, packet);

            let length = new_state.position.saturating_sub(start_position);
            if length == 0 {
                continue;
            }
            let cost = (perplexity / length as u64) as f32;
            let entry = TopKEntry { packet, cost };

            let count = self.count();
            if count < self.size {
                let pos = count;
                self.entries[pos] = entry;
                if let Some(costs) = registry()
                    .lock()
                    .expect("top-k cost registry poisoned")
                    .get_mut(&heap_key)
                {
                    costs[pos] = cost;
                }
                let _ = self.heap.insert(pos as u32);
                continue;
            }

            let Some(maximum) = self.heap.maximum() else {
                continue;
            };
            let maximum = maximum as usize;
            if cost <= self.entries[maximum].cost {
                self.entries[maximum] = entry;
                if let Some(costs) = registry()
                    .lock()
                    .expect("top-k cost registry poisoned")
                    .get_mut(&heap_key)
                {
                    costs[maximum] = cost;
                }
                let _ = self.heap.update_maximum();
            }
        }
    }
    pub fn pop(&mut self) -> Option<LZMAPacket> {
        let maximum = self.heap.maximum()? as usize;
        let packet = self.entries[maximum].packet;
        let _ = self.heap.remove_maximum();
        Some(packet)
    }
}
