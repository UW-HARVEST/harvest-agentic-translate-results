use crate::encoder_interface::EncoderInterface;
use crate::perplexity_table::LOG2_LOOKUP;
use crate::probability::Prob;

const DECIMAL_PLACES: u32 = 11;

/// A real Rust encoder that accumulates "perplexity" — a measure
/// proportional to the number of bits the range encoder would emit.
pub struct PerplexityEncoder<'a> {
    pub perplexity: &'a mut u64,
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

/// API-compatible helper. The C function set up function pointers on the
/// supplied `EncoderInterface`. In safe Rust, the caller should construct a
/// `PerplexityEncoder` and use it via `&mut dyn EncoderInterface` directly,
/// so this helper is a no-op kept for API parity.
pub fn perplexity_encoder_new(_enc: &mut dyn EncoderInterface, _perplexity: &mut u64) {
    // no-op — see doc comment.
}
