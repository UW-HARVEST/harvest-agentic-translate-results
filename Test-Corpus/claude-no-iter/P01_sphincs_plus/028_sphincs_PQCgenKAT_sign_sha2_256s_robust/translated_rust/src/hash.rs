// Hash function dispatch — picks the right backend based on Cargo features.

use crate::context::SpxCtx;

#[cfg(feature = "haraka")]
pub use crate::hash_haraka::{
    gen_message_random, hash_message, initialize_hash_function as init_hash_inner, prf_addr,
};
#[cfg(feature = "sha2")]
pub use crate::hash_sha2::{
    gen_message_random, hash_message, initialize_hash_function as init_hash_inner, prf_addr,
};
#[cfg(feature = "shake")]
pub use crate::hash_shake::{
    gen_message_random, hash_message, initialize_hash_function as init_hash_inner, prf_addr,
};
#[cfg(feature = "blake")]
pub use crate::hash_blake::{
    gen_message_random, hash_message, initialize_hash_function as init_hash_inner, prf_addr,
};

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    init_hash_inner(ctx);
}
