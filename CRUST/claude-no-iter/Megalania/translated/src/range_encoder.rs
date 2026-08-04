use crate::encoder_interface::EncoderInterface;
use crate::output_interface::OutputInterface;
use crate::probability::{Prob, NUM_BIT_MODEL_TOTAL_BITS};

const TOP_MASK: u32 = 0xFF000000;

/// A range encoder that wraps an `OutputInterface` and produces an LZMA bit stream.
pub struct RangeEncoder<'a> {
    output: &'a mut dyn OutputInterface,
    low: u64,
    range: u32,
    cache: u8,
    cache_size: u64,
}

impl<'a> RangeEncoder<'a> {
    pub fn new(output: &'a mut dyn OutputInterface) -> Self {
        Self {
            output,
            low: 0,
            range: 0xFFFFFFFF,
            cache: 0,
            cache_size: 1,
        }
    }

    fn shift_low(&mut self) {
        let high_bytes = (self.low >> 32) as u32;
        let low_bytes = (self.low & 0xFFFFFFFF) as u32;
        if low_bytes < TOP_MASK || high_bytes != 0 {
            let mut temp = self.cache;
            loop {
                let out_byte = temp.wrapping_add((high_bytes & 0xFF) as u8);
                if !self.output.write(&[out_byte]) {
                    eprintln!("could not write: {:02x}", out_byte);
                }
                temp = 0xFF;
                self.cache_size -= 1;
                if self.cache_size == 0 {
                    break;
                }
            }
            self.cache = ((self.low >> 24) & 0xFF) as u8;
        }
        self.cache_size += 1;
        self.low = ((low_bytes as u64) << 8) & 0xFFFFFFFFFF; // keep up to 40 bits
    }

    fn flush_data(&mut self) {
        for _ in 0..5 {
            self.shift_low();
        }
    }
}

impl<'a> EncoderInterface for RangeEncoder<'a> {
    fn encode_bit(&mut self, bit: bool, prob: Prob) {
        let new_bound = (self.range >> NUM_BIT_MODEL_TOTAL_BITS) * (prob as u32);
        if bit {
            self.low = self.low.wrapping_add(new_bound as u64);
            self.range -= new_bound;
        } else {
            self.range = new_bound;
        }
        while (self.range & TOP_MASK) == 0 {
            self.range <<= 8;
            self.shift_low();
        }
    }

    fn encode_direct_bits(&mut self, bits: u32, num_bits: u32) {
        let mut num_bits = num_bits;
        loop {
            let bit = (bits & (1u32 << (num_bits - 1))) != 0;
            self.range >>= 1;
            if bit {
                self.low = self.low.wrapping_add(self.range as u64);
            }
            if (self.range & TOP_MASK) == 0 {
                self.range <<= 8;
                self.shift_low();
            }
            num_bits -= 1;
            if num_bits == 0 {
                break;
            }
        }
    }
}

impl<'a> Drop for RangeEncoder<'a> {
    fn drop(&mut self) {
        self.flush_data();
    }
}

/// Configures an existing encoder interface to behave like a range encoder.
///
/// In C this set the function pointers and `private_data` of the
/// `EncoderInterface` struct. The Rust trait equivalent is to create a
/// `RangeEncoder` directly. This function is preserved as a no-op for
/// API parity.
pub fn range_encoder_new(_enc: &mut dyn EncoderInterface, _output: &mut dyn OutputInterface) {
    // Intentionally a no-op. Use `RangeEncoder::new` to obtain a concrete
    // range encoder.
}

/// Flushes the range encoder. Provided for API parity; the concrete
/// `RangeEncoder` automatically flushes when dropped.
pub fn range_encoder_free(_enc: &mut dyn EncoderInterface) {
    // Intentionally a no-op.
}
