// Thash dispatch module - delegates to the active backend + variant

#[cfg(all(feature = "shake", feature = "robust"))]
pub use crate::shake::thash_shake_robust::thash;

#[cfg(all(feature = "shake", feature = "simple"))]
pub use crate::shake::thash_shake_simple::thash;

#[cfg(all(feature = "sha2", feature = "robust"))]
pub use crate::sha2::thash_sha2_robust::thash;

#[cfg(all(feature = "sha2", feature = "simple"))]
pub use crate::sha2::thash_sha2_simple::thash;

#[cfg(all(feature = "blake", feature = "robust"))]
pub use crate::blake::thash_blake_robust::thash;

#[cfg(all(feature = "blake", feature = "simple"))]
pub use crate::blake::thash_blake_simple::thash;

#[cfg(all(feature = "haraka", feature = "robust"))]
pub use crate::haraka::thash_haraka_robust::thash;

#[cfg(all(feature = "haraka", feature = "simple"))]
pub use crate::haraka::thash_haraka_simple::thash;
