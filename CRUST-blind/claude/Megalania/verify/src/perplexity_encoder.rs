use crate::encoder_interface::EncoderInterface;
use crate::perplexity_table::LOG2_LOOKUP;
use crate::probability::Prob;

const DECIMAL_PLACES: u32 = 11;

/// A perplexity-based encoder that maintains a running cost (in 11-bit fixed
/// point bits) instead of writing out actual encoded data.
pub struct PerplexityEncoder<'a> {
    pub perplexity: &'a mut u64,
}

impl<'a> EncoderInterface for PerplexityEncoder<'a> {
    fn encode_bit(&mut self, bit: bool, prob: Prob) {
        let idx = if bit {
            (2048u32 - prob as u32) as usize
        } else {
            prob as usize
        };
        *self.perplexity += LOG2_LOOKUP[idx];
    }

    fn encode_direct_bits(&mut self, _bits: u32, num_bits: u32) {
        *self.perplexity += (num_bits as u64) << DECIMAL_PLACES;
    }
}

/// Compatibility shim for the C-style API. In Rust, prefer constructing
/// `PerplexityEncoder` directly.
pub fn perplexity_encoder_new(_enc: &mut dyn EncoderInterface, _perplexity: &mut u64) {
    // No-op: in Rust we use trait objects, so users build PerplexityEncoder directly.
}
