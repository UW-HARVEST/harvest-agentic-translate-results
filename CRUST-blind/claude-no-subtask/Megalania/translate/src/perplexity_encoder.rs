use crate::encoder_interface::EncoderInterface;
use crate::perplexity_table::LOG2_LOOKUP;
use crate::probability::Prob;

const DECIMAL_PLACES: u32 = 11;

/// Concrete perplexity encoder. Records the number of fractional bits required
/// to encode each bit given its probability.
pub struct PerplexityEncoder<'a> {
    perplexity: &'a mut u64,
}

impl<'a> PerplexityEncoder<'a> {
    pub fn new(perplexity: &'a mut u64) -> Self {
        PerplexityEncoder { perplexity }
    }
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

/// API-parity stub. In the original C code this attaches function pointers
/// and a perplexity pointer to an `EncoderInterface` value. In the Rust
/// translation users should construct a `PerplexityEncoder` directly and
/// pass it where a `&mut dyn EncoderInterface` is required.
pub fn perplexity_encoder_new(_enc: &mut dyn EncoderInterface, _perplexity: &mut u64) {
    // intentionally a no-op
}
