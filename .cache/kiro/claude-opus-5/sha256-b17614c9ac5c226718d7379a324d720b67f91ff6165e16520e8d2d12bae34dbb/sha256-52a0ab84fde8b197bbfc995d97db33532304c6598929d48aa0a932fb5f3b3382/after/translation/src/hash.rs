//! `app/include/hash.h`: forwards to the selected backend's `hash_*.c`.

pub use crate::backend::active::hash::{
    gen_message_random, hash_message, initialize_hash_function, prf_addr,
};
