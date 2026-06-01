// Hash backend modules

#[cfg(feature = "haraka")]
pub mod haraka;

#[cfg(feature = "sha2")]
pub mod sha2;

#[cfg(feature = "shake")]
pub mod shake;

#[cfg(feature = "blake")]
pub mod blake;
