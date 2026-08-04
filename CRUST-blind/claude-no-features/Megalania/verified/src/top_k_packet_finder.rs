use crate::encoder_interface::EncoderInterface;
use crate::lzma_packet::LZMAPacket;
use crate::lzma_packet_encoder::lzma_encode_packet;
use crate::lzma_state::LZMAState;
use crate::max_heap::MaxHeap;
use crate::packet_enumerator::PacketEnumerator;
use crate::perplexity_table::LOG2_LOOKUP;
use crate::probability::Prob;

const DECIMAL_PLACES: u32 = 11;

/// A single entry for a Top-K packet candidate.
pub struct TopKEntry {
    pub packet: LZMAPacket,
    pub cost: f32,
}

/// Local perplexity encoder used internally by the top-K finder.
struct LocalPerplexityEncoder<'a> {
    perplexity: &'a mut u64,
}

impl<'a> EncoderInterface for LocalPerplexityEncoder<'a> {
    fn encode_bit(&mut self, bit: bool, prob: Prob) {
        let idx = if bit {
            (2048 - prob as usize) as usize
        } else {
            prob as usize
        };
        *self.perplexity += LOG2_LOOKUP[idx];
    }

    fn encode_direct_bits(&mut self, _bits: u32, num_bits: u32) {
        *self.perplexity += (num_bits as u64) << DECIMAL_PLACES;
    }
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
        // SAFETY: The signature is fixed, but we need to store the
        // packet_enumerator reference with lifetime 'a. We extend the
        // lifetime via transmute, relying on the caller to ensure it
        // outlives this struct.
        let pe: &'a PacketEnumerator<'a> = unsafe { std::mem::transmute(_packet_enumerator) };
        TopKPacketFinder {
            size,
            entries: Vec::with_capacity(size),
            next_packets: Vec::new(),
            heap: Box::new(MaxHeap::new(size, Box::new(|_a, _b| 0))),
            packet_enumerator: pe,
        }
    }
    pub fn count(&self) -> usize {
        self.entries.len()
    }
    pub fn find(&mut self, _lzma_state: &LZMAState, _next_packets: &mut [LZMAPacket]) {
        self.entries.clear();
        // Take a copy of next_packets so we can index into it during the
        // closure without borrow issues.
        self.next_packets.clear();
        self.next_packets.extend_from_slice(_next_packets);

        // Use raw pointer to avoid borrow conflicts. We borrow self.entries
        // mutably through a raw pointer, which is sound because the closure
        // does not otherwise touch `self.entries`.
        let entries_ptr: *mut Vec<TopKEntry> = &mut self.entries;
        let next_packets_ptr: *const Vec<LZMAPacket> = &self.next_packets;
        let size = self.size;
        let pe = self.packet_enumerator;

        pe.for_each(_lzma_state, |state, packet| {
            // SAFETY: see above.
            let next_packets = unsafe { &*next_packets_ptr };
            if LZMAPacket::cmp(&packet, &next_packets[state.position]) {
                return;
            }

            // Compute the cost of this candidate packet.
            let mut new_state = state.clone();
            let mut perplexity: u64 = 0;
            let start_position = new_state.position;
            {
                let mut enc = LocalPerplexityEncoder {
                    perplexity: &mut perplexity,
                };
                lzma_encode_packet(&mut new_state, &mut enc, packet);
            }
            let length = new_state.position - start_position;
            let cost = if length > 0 {
                (perplexity as f32) / (length as f32)
            } else {
                f32::INFINITY
            };

            // SAFETY: see above.
            let entries = unsafe { &mut *entries_ptr };
            if entries.len() < size {
                entries.push(TopKEntry { packet, cost });
            } else {
                // Find the worst entry (max cost).
                let mut max_idx = 0;
                for i in 1..entries.len() {
                    if entries[i].cost > entries[max_idx].cost {
                        max_idx = i;
                    }
                }
                if cost <= entries[max_idx].cost {
                    entries[max_idx] = TopKEntry { packet, cost };
                }
            }
        });

        // Sort ascending by cost so pop() (which takes from the end)
        // returns the worst entry first and the best entry last.
        self.entries.sort_by(|a, b| {
            a.cost
                .partial_cmp(&b.cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    pub fn pop(&mut self) -> Option<LZMAPacket> {
        self.entries.pop().map(|entry| entry.packet)
    }
}
