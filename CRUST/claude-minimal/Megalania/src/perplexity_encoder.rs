use crate::encoder_interface::EncoderInterface;
use crate::perplexity_table::LOG2_LOOKUP;
use crate::probability::Prob;

const DECIMAL_PLACES: u32 = 11;

/// PerplexityEncoder accumulates the cost (perplexity) of encoded bits
/// into the supplied counter. It mirrors the C `perplexity_encoder_*`
/// functions, which provide an EncoderInterface that updates a uint64_t
/// pointer.
pub struct PerplexityEncoder<'a> {
    perplexity: &'a mut u64,
}

impl<'a> PerplexityEncoder<'a> {
    pub fn new(perplexity: &'a mut u64) -> Self {
        Self { perplexity }
    }
}

impl<'a> EncoderInterface for PerplexityEncoder<'a> {
    fn encode_bit(&mut self, bit: bool, prob: Prob) {
        let idx = if bit { 2048 - prob as usize } else { prob as usize };
        *self.perplexity += LOG2_LOOKUP[idx];
    }
    fn encode_direct_bits(&mut self, _bits: u32, num_bits: u32) {
        *self.perplexity += (num_bits as u64) << DECIMAL_PLACES;
    }
}

/// API-compatible shim for the C `perplexity_encoder_new`. In Rust, callers
/// should construct a `PerplexityEncoder` directly.
pub fn perplexity_encoder_new(_enc: &mut dyn EncoderInterface, _perplexity: &mut u64) {
    // No direct equivalent in safe Rust.
}
