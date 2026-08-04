use crate::encoder_interface::EncoderInterface;
use crate::perplexity_table::LOG2_LOOKUP;
use std::sync::{Mutex, OnceLock};

fn encoder_key(enc: &mut dyn EncoderInterface) -> usize {
    enc as *mut dyn EncoderInterface as *mut () as usize
}

fn registry() -> &'static Mutex<std::collections::HashMap<usize, usize>> {
    static REGISTRY: OnceLock<Mutex<std::collections::HashMap<usize, usize>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

pub(crate) fn try_encode_bit(enc: &mut dyn EncoderInterface, bit: bool, prob: crate::probability::Prob) -> bool {
    let key = encoder_key(enc);
    let ptr = {
        let guard = registry().lock().expect("perplexity registry poisoned");
        guard.get(&key).copied()
    };
    if let Some(ptr) = ptr {
        let amount = LOG2_LOOKUP[if bit { 2048 - prob as usize } else { prob as usize }];
        // SAFETY: `perplexity_encoder_new` stores the caller-provided `&mut u64`.
        // The original C API mutates that value through the encoder interface.
        unsafe {
            *(ptr as *mut u64) += amount;
        }
        true
    } else {
        false
    }
}

pub(crate) fn try_encode_direct_bits(
    enc: &mut dyn EncoderInterface,
    num_bits: usize,
) -> bool {
    let key = encoder_key(enc);
    let ptr = {
        let guard = registry().lock().expect("perplexity registry poisoned");
        guard.get(&key).copied()
    };
    if let Some(ptr) = ptr {
        // SAFETY: see `try_encode_bit`.
        unsafe {
            *(ptr as *mut u64) += (num_bits as u64) << 11;
        }
        true
    } else {
        false
    }
}

pub fn perplexity_encoder_new(enc: &mut dyn EncoderInterface, perplexity: &mut u64) {
    registry()
        .lock()
        .expect("perplexity registry poisoned")
        .insert(encoder_key(enc), perplexity as *mut u64 as usize);
}
