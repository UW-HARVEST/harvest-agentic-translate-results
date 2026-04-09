// Hash function dispatch module - delegates to the active backend

#[cfg(feature = "shake")]
pub use crate::shake::hash_shake::{initialize_hash_function, prf_addr, gen_message_random, hash_message};

#[cfg(feature = "sha2")]
pub use crate::sha2::hash_sha2::{initialize_hash_function, prf_addr, gen_message_random, hash_message};

#[cfg(feature = "blake")]
pub use crate::blake::hash_blake::{initialize_hash_function, prf_addr, gen_message_random, hash_message};

#[cfg(feature = "haraka")]
pub use crate::haraka::hash_haraka::{initialize_hash_function, prf_addr, gen_message_random, hash_message};
