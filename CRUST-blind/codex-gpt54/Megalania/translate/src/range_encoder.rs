use crate::encoder_interface::EncoderInterface;
use crate::output_interface::OutputInterface;
use crate::probability::{Prob, NUM_BIT_MODEL_TOTAL_BITS};
use std::sync::{Mutex, OnceLock};

const TOP_MASK: u32 = 0xFF00_0000;

struct RangeEncoderData {
    output_ptr: (usize, usize),
    low: u64,
    range: u32,
    cache: u8,
    cache_size: u64,
}

fn encoder_key(enc: &mut dyn EncoderInterface) -> usize {
    enc as *mut dyn EncoderInterface as *mut () as usize
}

fn registry() -> &'static Mutex<std::collections::HashMap<usize, RangeEncoderData>> {
    static REGISTRY: OnceLock<Mutex<std::collections::HashMap<usize, RangeEncoderData>>> =
        OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

fn shift_low(data: &mut RangeEncoderData) {
    let high_bytes = (data.low >> 32) as u32;
    let low_bytes = (data.low & 0xFFFF_FFFF) as u32;
    if low_bytes < TOP_MASK || high_bytes != 0 {
        let mut temp = data.cache;
        while data.cache_size != 0 {
            let out_byte = temp.wrapping_add((high_bytes & 0xFF) as u8);
            // SAFETY: `range_encoder_new` stores the output trait object pointer
            // for use during later encode calls, matching the C interface contract.
            let output_ptr: *mut dyn OutputInterface = unsafe { std::mem::transmute(data.output_ptr) };
            // SAFETY: the stored output pointer remains valid for the encoder lifetime.
            let output = unsafe { &mut *output_ptr };
            let _ = crate::file_output::write_output(output, &[out_byte]);
            temp = 0xFF;
            data.cache_size -= 1;
        }
        data.cache = ((data.low >> 24) & 0xFF) as u8;
    }
    data.cache_size += 1;
    data.low = (low_bytes as u64) << 8;
}

pub(crate) fn try_encode_bit(enc: &mut dyn EncoderInterface, bit: bool, prob: Prob) -> bool {
    let key = encoder_key(enc);
    let mut guard = registry().lock().expect("range encoder registry poisoned");
    let Some(data) = guard.get_mut(&key) else {
        return false;
    };

    let new_bound = (data.range >> NUM_BIT_MODEL_TOTAL_BITS) * prob as u32;
    if bit {
        data.low += new_bound as u64;
        data.range -= new_bound;
    } else {
        data.range = new_bound;
    }
    while (data.range & TOP_MASK) == 0 {
        data.range <<= 8;
        shift_low(data);
    }
    true
}

pub(crate) fn try_encode_direct_bits(
    enc: &mut dyn EncoderInterface,
    bits: u32,
    num_bits: usize,
) -> bool {
    let key = encoder_key(enc);
    let mut guard = registry().lock().expect("range encoder registry poisoned");
    let Some(data) = guard.get_mut(&key) else {
        return false;
    };

    if num_bits == 0 {
        return true;
    }

    let mut remaining = num_bits;
    while remaining != 0 {
        let bit = (bits & (1 << (remaining - 1))) != 0;
        data.range >>= 1;
        if bit {
            data.low += data.range as u64;
        }
        if (data.range & TOP_MASK) == 0 {
            data.range <<= 8;
            shift_low(data);
        }
        remaining -= 1;
    }
    true
}

pub fn range_encoder_new(enc: &mut dyn EncoderInterface, output: &mut dyn OutputInterface) {
    let output_ptr: *mut dyn OutputInterface = output;
    registry()
        .lock()
        .expect("range encoder registry poisoned")
        .insert(
            encoder_key(enc),
            RangeEncoderData {
                // SAFETY: splitting a fat pointer into its raw components preserves it for later use.
                output_ptr: unsafe { std::mem::transmute(output_ptr) },
                low: 0,
                range: 0xFFFF_FFFF,
                cache: 0,
                cache_size: 1,
            },
        );
}
pub fn range_encoder_free(enc: &mut dyn EncoderInterface) {
    let key = encoder_key(enc);
    let mut guard = registry().lock().expect("range encoder registry poisoned");
    if let Some(mut data) = guard.remove(&key) {
        for _ in 0..5 {
            shift_low(&mut data);
        }
    }
}
