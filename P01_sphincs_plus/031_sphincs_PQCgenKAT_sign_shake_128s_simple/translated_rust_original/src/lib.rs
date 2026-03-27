#![allow(non_snake_case, non_upper_case_globals, unused_assignments, clippy::needless_range_loop)]

pub mod params;
pub mod context;
pub mod address;
pub mod utils;
pub mod wots;
pub mod fors;
pub mod merkle;
pub mod sign;
pub mod wotsx1;
pub mod utilsx1;
pub mod forsx1;
pub mod rng;
pub mod randombytes;

// Hash backend modules
#[cfg(feature = "blake")]
pub mod blake;
#[cfg(feature = "sha2")]
pub mod sha2;
#[cfg(feature = "shake")]
pub mod shake;
#[cfg(feature = "haraka")]
pub mod haraka;

// Re-export the hash/thash functions from the selected backend
// Each backend module must provide: initialize_hash_function, prf_addr, gen_message_random, hash_message, thash
pub mod hash {
    #[cfg(feature = "blake")]
    pub use crate::blake::hash_blake::*;
    #[cfg(feature = "sha2")]
    pub use crate::sha2::hash_sha2::*;
    #[cfg(feature = "shake")]
    pub use crate::shake::hash_shake::*;
    #[cfg(feature = "haraka")]
    pub use crate::haraka::hash_haraka::*;
}

pub mod thash {
    #[cfg(all(feature = "blake", feature = "simple"))]
    pub use crate::blake::thash_blake_simple::*;
    #[cfg(all(feature = "blake", feature = "robust"))]
    pub use crate::blake::thash_blake_robust::*;
    #[cfg(all(feature = "sha2", feature = "simple"))]
    pub use crate::sha2::thash_sha2_simple::*;
    #[cfg(all(feature = "sha2", feature = "robust"))]
    pub use crate::sha2::thash_sha2_robust::*;
    #[cfg(all(feature = "shake", feature = "simple"))]
    pub use crate::shake::thash_shake_simple::*;
    #[cfg(all(feature = "shake", feature = "robust"))]
    pub use crate::shake::thash_shake_robust::*;
    #[cfg(all(feature = "haraka", feature = "simple"))]
    pub use crate::haraka::thash_haraka_simple::*;
    #[cfg(all(feature = "haraka", feature = "robust"))]
    pub use crate::haraka::thash_haraka_robust::*;
}
