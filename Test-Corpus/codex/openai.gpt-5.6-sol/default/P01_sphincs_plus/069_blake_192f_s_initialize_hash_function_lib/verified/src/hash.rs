//! Backend-agnostic re-export of the active hash backend's `hash.c` interface.

#[cfg(spx_backend = "sha2")]
pub use crate::sha2_hash::{gen_message_random, hash_message, initialize_hash_function, prf_addr};
#[cfg(spx_backend = "shake")]
pub use crate::shake_hash::{gen_message_random, hash_message, initialize_hash_function, prf_addr};
#[cfg(spx_backend = "haraka")]
pub use crate::haraka_hash::{gen_message_random, hash_message, initialize_hash_function, prf_addr};
#[cfg(spx_backend = "blake")]
pub use crate::blake_hash::{gen_message_random, hash_message, initialize_hash_function, prf_addr};
