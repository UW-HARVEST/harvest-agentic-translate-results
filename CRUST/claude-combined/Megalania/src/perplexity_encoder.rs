use crate::encoder_interface::EncoderInterface;
use crate::perplexity_table::LOG2_LOOKUP;
use crate::probability::Prob;
use std::cell::RefCell;
use std::rc::Rc;

const DECIMAL_PLACES: u32 = 11;

pub struct PerplexityEncoder {
    pub perplexity: Rc<RefCell<u64>>,
}

impl EncoderInterface for PerplexityEncoder {
    fn encode_bit(&mut self, bit: bool, prob: Prob) {
        let idx = if bit {
            (2048 - prob as i32) as usize
        } else {
            prob as usize
        };
        *self.perplexity.borrow_mut() += LOG2_LOOKUP[idx];
    }

    fn encode_direct_bits(&mut self, _bits: u32, num_bits: u32) {
        *self.perplexity.borrow_mut() += (num_bits as u64) << DECIMAL_PLACES;
    }
}

impl PerplexityEncoder {
    pub fn new(perplexity: Rc<RefCell<u64>>) -> Self {
        Self { perplexity }
    }
}

pub fn perplexity_encoder_new(_enc: &mut dyn EncoderInterface, _perplexity: &mut u64) {
    // Stub: in C this initialized function pointers and a private data pointer.
    // In Rust, construct PerplexityEncoder directly and pass it as &mut dyn EncoderInterface.
}
