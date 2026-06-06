use crate::encoder_interface::EncoderInterface;
use crate::perplexity_table::LOG2_LOOKUP;
use crate::probability::Prob;

const DECIMAL_PLACES: u32 = 11;

/// A perplexity encoder records the number of bits required to encode each bit,
/// using a log2 lookup table. The internal value is a 53.11 fixed-point number.
pub struct PerplexityEncoder<'a> {
    pub perplexity: &'a mut u64,
}

impl<'a> PerplexityEncoder<'a> {
    pub fn new(perplexity: &'a mut u64) -> Self {
        PerplexityEncoder { perplexity }
    }
}

impl<'a> EncoderInterface for PerplexityEncoder<'a> {
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

/// In the original C code, this populates an EncoderInterface struct with
/// function pointers. In idiomatic Rust, prefer `PerplexityEncoder::new(perplexity)`.
pub fn perplexity_encoder_new(_enc: &mut dyn EncoderInterface, _perplexity: &mut u64) {
    // No-op: prefer PerplexityEncoder::new(perplexity).
}
