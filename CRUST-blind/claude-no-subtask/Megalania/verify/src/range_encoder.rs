use crate::encoder_interface::EncoderInterface;
use crate::output_interface::OutputInterface;
use crate::probability::{Prob, NUM_BIT_MODEL_TOTAL_BITS};

const TOP_MASK: u32 = 0xFF000000;

/// Concrete range encoder used during the final emit pass.
pub struct RangeEncoder<'a> {
    output: &'a mut dyn OutputInterface,
    low: u64,
    range: u32,
    cache: u8,
    cache_size: u64,
}

impl<'a> RangeEncoder<'a> {
    pub fn new(output: &'a mut dyn OutputInterface) -> Self {
        RangeEncoder {
            output,
            low: 0,
            range: 0xFFFFFFFF,
            cache: 0,
            cache_size: 1,
        }
    }

    fn shift_low(&mut self) {
        let high_bytes: u32 = (self.low >> 32) as u32;
        let low_bytes: u32 = (self.low & 0xFFFFFFFF) as u32;
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
        self.low = (low_bytes as u64) << 8;
    }

    /// Flushes the remaining cached bytes. The C code calls
    /// `range_encoder_flush_data` from `range_encoder_free`. In Rust we expose
    /// this as `flush` and call it explicitly when finished encoding.
    pub fn flush(&mut self) {
        for _ in 0..5 {
            self.shift_low();
        }
    }
}

impl<'a> EncoderInterface for RangeEncoder<'a> {
    fn encode_bit(&mut self, bit: bool, prob: Prob) {
        let new_bound: u32 = (self.range >> NUM_BIT_MODEL_TOTAL_BITS) * (prob as u32);
        if bit {
            self.low += new_bound as u64;
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
        let mut n = num_bits;
        loop {
            let bit = (bits & (1u32 << (n - 1))) != 0;
            self.range >>= 1;
            if bit {
                self.low += self.range as u64;
            }
            if (self.range & TOP_MASK) == 0 {
                self.range <<= 8;
                self.shift_low();
            }
            n -= 1;
            if n == 0 {
                break;
            }
        }
    }
}

/// API-parity stub for the C function. In Rust, callers should construct a
/// `RangeEncoder` directly with `RangeEncoder::new(output)`.
pub fn range_encoder_new(_enc: &mut dyn EncoderInterface, _output: &mut dyn OutputInterface) {
    // intentionally a no-op
}

/// Flushes the remaining cached bytes of a range encoder. In Rust, callers
/// should call `RangeEncoder::flush()` on the concrete encoder. This stub
/// exists only for API parity.
pub fn range_encoder_free(_enc: &mut dyn EncoderInterface) {
    // intentionally a no-op
}
