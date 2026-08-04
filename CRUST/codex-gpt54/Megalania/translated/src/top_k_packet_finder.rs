use crate::encoder_interface::EncoderInterface;
use crate::lzma_packet::LZMAPacket;
use crate::lzma_state::LZMAState;
use crate::lzma_packet_encoder::lzma_encode_packet;
use crate::max_heap::MaxHeap;
use crate::packet_enumerator::PacketEnumerator;
use crate::perplexity_table::LOG2_LOOKUP;
use crate::probability::Prob;
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
    pub fn new(size: usize, _packet_enumerator: &PacketEnumerator) -> Self {
        let packet_enumerator: &'a PacketEnumerator<'a> =
            unsafe { std::mem::transmute(_packet_enumerator) };
        Self {
            size,
            entries: Vec::with_capacity(size),
            next_packets: Vec::new(),
            heap: Box::new(MaxHeap::new(size, Box::new(|a, b| (a as i32) - (b as i32)))),
            packet_enumerator,
        }
    }
    pub fn count(&self) -> usize {
        self.entries.len()
    }
    pub fn find(&mut self, _lzma_state: &LZMAState, _next_packets: &mut [LZMAPacket]) {
        self.next_packets.clear();
        self.next_packets.extend_from_slice(_next_packets);
        self.entries.clear();
        self.heap.clear();

        let next_packets = self.next_packets.clone();
        let size = self.size;
        let entries = std::cell::RefCell::new(Vec::with_capacity(size));

        self.packet_enumerator.for_each(_lzma_state, |state, packet| {
            if next_packets
                .get(state.position)
                .is_some_and(|next_packet| LZMAPacket::cmp(&packet, next_packet))
            {
                return;
            }

            let mut new_state = state.clone();
            let start_position = new_state.position;
            let mut encoder = PerplexityCounter { perplexity: 0 };
            lzma_encode_packet(&mut new_state, &mut encoder, packet);

            let length = new_state.position - start_position;
            if length == 0 {
                return;
            }

            let entry = TopKEntry {
                packet,
                cost: (encoder.perplexity / length as u64) as f32,
            };

            let mut entries = entries.borrow_mut();
            if entries.len() < size {
                entries.push(entry);
            } else if let Some((max_index, max_entry)) = entries
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.cost.partial_cmp(&b.cost).unwrap())
            {
                if entry.cost <= max_entry.cost {
                    entries[max_index] = entry;
                }
            }
        });

        self.entries = entries.into_inner();
        self.entries
            .sort_by(|a, b| b.cost.partial_cmp(&a.cost).unwrap());
    }
    pub fn pop(&mut self) -> Option<LZMAPacket> {
        if self.entries.is_empty() {
            None
        } else {
            Some(self.entries.remove(0).packet)
        }
    }
}

struct PerplexityCounter {
    perplexity: u64,
}

impl EncoderInterface for PerplexityCounter {
    fn encode_bit(&mut self, bit: bool, prob: Prob) {
        self.perplexity += LOG2_LOOKUP[if bit {
            (2048 - prob) as usize
        } else {
            prob as usize
        }];
    }

    fn encode_direct_bits(&mut self, _bits: u32, num_bits: u32) {
        self.perplexity += (num_bits as u64) << 11;
    }
}
