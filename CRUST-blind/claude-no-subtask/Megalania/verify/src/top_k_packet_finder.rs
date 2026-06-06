use crate::encoder_interface::EncoderInterface;
use crate::lzma_packet::LZMAPacket;
use crate::lzma_packet_encoder::lzma_encode_packet;
use crate::lzma_state::LZMAState;
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

/// Local perplexity encoder used by the top-k finder. Duplicated here so we
/// can keep the public `EncoderInterface` trait simple while still allowing a
/// concrete encoder to be created on the stack.
struct LocalPerplexityEncoder<'a> {
    perplexity: &'a mut u64,
}
impl<'a> EncoderInterface for LocalPerplexityEncoder<'a> {
    fn encode_bit(&mut self, bit: bool, prob: Prob) {
        let idx = if bit {
            (2048u32 - prob as u32) as usize
        } else {
            prob as usize
        };
        *self.perplexity += LOG2_LOOKUP[idx];
    }
    fn encode_direct_bits(&mut self, _bits: u32, num_bits: u32) {
        *self.perplexity += (num_bits as u64) << 11;
    }
}

impl<'a> TopKPacketFinder<'a> {
    pub fn new(size: usize, packet_enumerator: &'a PacketEnumerator<'a>) -> Self {
        // The MaxHeap is constructed with a comparator that, in the C version,
        // dereferences a pointer back to the finder. Since we don't actually
        // use this heap (we sort an entries vector instead), we give the heap a
        // benign comparator that compares the raw u32 values. This preserves
        // the public field but keeps the implementation simple and safe.
        let heap = Box::new(MaxHeap::new(
            size,
            Box::new(|a: u32, b: u32| (a as i64 - b as i64).signum() as i32),
        ));
        TopKPacketFinder {
            size,
            entries: Vec::new(),
            next_packets: Vec::new(),
            heap,
            packet_enumerator,
        }
    }
    pub fn count(&self) -> usize {
        self.entries.len()
    }
    pub fn find(&mut self, lzma_state: &LZMAState, next_packets: &mut [LZMAPacket]) {
        self.entries.clear();
        let max_size = self.size;
        let mut collected: Vec<TopKEntry> = Vec::new();

        // We need to use the packet enumerator with a callback that captures
        // mutable state. Since `for_each` takes `Fn`, we use interior
        // mutability via RefCell.
        let collected_cell = std::cell::RefCell::new(&mut collected);
        let next_packets_ref: &[LZMAPacket] = next_packets;

        self.packet_enumerator
            .for_each(lzma_state, |state, packet| {
                if LZMAPacket::cmp(&packet, &next_packets_ref[state.position]) {
                    return;
                }

                // Simulate encoding the packet from a copy of the LZMA state
                // and measure the per-byte cost (perplexity).
                let mut new_state: LZMAState = state.clone();
                let mut perplexity: u64 = 0;
                let start_position = new_state.position;
                {
                    let mut enc = LocalPerplexityEncoder {
                        perplexity: &mut perplexity,
                    };
                    lzma_encode_packet(&mut new_state, &mut enc, packet);
                }
                let length = new_state.position - start_position;
                if length == 0 {
                    return;
                }
                let cost = (perplexity as f32) / (length as f32);
                collected_cell.borrow_mut().push(TopKEntry { packet, cost });
            });

        // Keep the K entries with the lowest cost.
        // Sort ascending by cost so that the smallest is first.
        collected.sort_by(|a, b| {
            a.cost
                .partial_cmp(&b.cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if collected.len() > max_size {
            collected.truncate(max_size);
        }
        // We want `pop` to return the worst entry first and the best last.
        // Reverse so that the worst is at the end (popped first).
        collected.reverse();
        // After reverse: [worst, ..., best]. pop() returns best first; we need
        // worst first. So instead store as [best, worst] order and pop from
        // end.
        // Actually: we want pop to behave like the C max-heap pop:
        //   - first pop returns the highest-cost (worst) entry
        //   - last pop returns the lowest-cost (best) entry
        // So we want pop_back to give worst first → store ascending [best..worst]
        collected.reverse(); // back to ascending [best..worst]
        self.entries = collected;
    }
    pub fn pop(&mut self) -> Option<LZMAPacket> {
        // pop the worst (highest-cost) entry first; entries is ascending so
        // the worst is at the end.
        self.entries.pop().map(|e| e.packet)
    }
}
