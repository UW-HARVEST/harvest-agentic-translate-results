//! Backend-agnostic re-export of the active backend's `thash`.

#[cfg(spx_backend = "sha2")]
pub use crate::sha2_thash::thash;
#[cfg(spx_backend = "shake")]
pub use crate::shake_thash::thash;
#[cfg(spx_backend = "haraka")]
pub use crate::haraka_thash::thash;
#[cfg(spx_backend = "blake")]
pub use crate::blake_thash::thash;
