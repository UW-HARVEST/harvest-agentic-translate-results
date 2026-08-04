// thash dispatch — picks the right backend based on Cargo features.

#[cfg(feature = "haraka")]
pub use crate::thash_haraka::thash;
#[cfg(feature = "sha2")]
pub use crate::thash_sha2::thash;
#[cfg(feature = "shake")]
pub use crate::thash_shake::thash;
#[cfg(feature = "blake")]
pub use crate::thash_blake::thash;
