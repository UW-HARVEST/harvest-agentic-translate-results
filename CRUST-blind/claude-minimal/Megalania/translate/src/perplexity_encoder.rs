use crate::encoder_interface::EncoderInterface;
use crate::perplexity_table::LOG2_LOOKUP;
use crate::probability::Prob;

const DECIMAL_PLACES: u32 = 11;

/// A perplexity encoder that records the exact number of bits needed to encode
/// a bit given the probability it will be zero. The perplexity is a 53.11
/// fixed-point value.
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

/// Equivalent to the C `perplexity_encoder_new` initializer.
pub fn perplexity_encoder_new(_enc: &mut dyn EncoderInterface, _perplexity: &mut u64) {
    // In Rust we cannot mutate the trait object's vtable; users should
    // instead construct a `PerplexityEncoder` directly. This function is a
    // no-op, kept for parity with the C API.
}
