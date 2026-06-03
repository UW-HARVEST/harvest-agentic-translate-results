use crate::encoder_interface::EncoderInterface;
use crate::perplexity_table::LOG2_LOOKUP;
use crate::probability::Prob;

const DECIMAL_PLACES: u32 = 11;

/// A concrete encoder that records LZMA perplexity (an upper bound on the
/// expected encoded bit length, in fixed point).
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

/// Configures an existing encoder interface to behave like a perplexity encoder.
///
/// In the C source this initializes the function-pointer fields and the
/// `private_data` pointer of an `EncoderInterface` struct. Rust traits do
/// not allow swapping the dynamic type of a `&mut dyn EncoderInterface`,
/// so this function is provided for API parity. Use [`PerplexityEncoder`]
/// directly when constructing a new encoder.
pub fn perplexity_encoder_new(_enc: &mut dyn EncoderInterface, _perplexity: &mut u64) {
    // Intentionally a no-op. See `PerplexityEncoder` above for the concrete
    // implementation used by the rest of the crate.
}
