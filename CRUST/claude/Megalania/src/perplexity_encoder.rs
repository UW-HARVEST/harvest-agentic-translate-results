use crate::encoder_interface::EncoderInterface;
use crate::perplexity_table::LOG2_LOOKUP;
use crate::probability::Prob;

const DECIMAL_PLACES: u32 = 11;

/// Encoder that accumulates a perplexity (entropy) score, used by the
/// optimizer to evaluate candidate packet sequences.
pub struct PerplexityEncoder {
    pub perplexity: u64,
}

impl PerplexityEncoder {
    pub fn new() -> Self {
        Self { perplexity: 0 }
    }
}

impl Default for PerplexityEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl EncoderInterface for PerplexityEncoder {
    fn encode_bit(&mut self, bit: bool, prob: Prob) {
        let idx = if bit { 2048 - prob as usize } else { prob as usize };
        self.perplexity += LOG2_LOOKUP[idx];
    }
    fn encode_direct_bits(&mut self, _bits: u32, num_bits: u32) {
        self.perplexity += (num_bits as u64) << DECIMAL_PLACES;
    }
}

/// Compatibility wrapper mirroring the C API.
/// Because we cannot rebind the trait object behind `enc`, this is a no-op.
/// Consumers should use the `PerplexityEncoder` struct directly.
pub fn perplexity_encoder_new(_enc: &mut dyn EncoderInterface, _perplexity: &mut u64) {
    // No-op shim. See comment above.
}
