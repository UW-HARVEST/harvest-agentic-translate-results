pub mod params;
pub mod context;
pub mod address;
pub mod utils;
pub mod utilsx1;
pub mod wots;
pub mod wotsx1;
pub mod fors;
pub mod merkle;
pub mod sign;
pub mod randombytes;
pub mod rng;

#[cfg(feature = "hash-sha2")]
pub mod sha2_hash;
#[cfg(feature = "hash-sha2")]
pub mod sha2_thash;

#[cfg(feature = "hash-shake")]
pub mod shake_hash;
#[cfg(feature = "hash-shake")]
pub mod shake_thash;

#[cfg(feature = "hash-blake")]
pub mod blake_hash;
#[cfg(feature = "hash-blake")]
pub mod blake_thash;

pub mod hash {
    #[cfg(feature = "hash-sha2")]
    pub use crate::sha2_hash::*;
    #[cfg(feature = "hash-shake")]
    pub use crate::shake_hash::*;
    #[cfg(feature = "hash-blake")]
    pub use crate::blake_hash::*;
}

pub mod thash {
    #[cfg(feature = "hash-sha2")]
    pub use crate::sha2_thash::*;
    #[cfg(feature = "hash-shake")]
    pub use crate::shake_thash::*;
    #[cfg(feature = "hash-blake")]
    pub use crate::blake_thash::*;
}
