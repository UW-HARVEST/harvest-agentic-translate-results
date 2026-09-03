#[cfg(feature = "blake")]
mod blake;
#[cfg(feature = "blake")]
pub use blake::*;

#[cfg(all(not(feature = "blake"), feature = "sha2"))]
mod sha2;
#[cfg(all(not(feature = "blake"), feature = "sha2"))]
pub use sha2::*;

#[cfg(all(not(feature = "blake"), not(feature = "sha2"), feature = "shake"))]
mod shake;
#[cfg(all(not(feature = "blake"), not(feature = "sha2"), feature = "shake"))]
pub use shake::*;

#[cfg(all(not(feature = "blake"), not(feature = "sha2"), not(feature = "shake")))]
mod haraka;
#[cfg(all(not(feature = "blake"), not(feature = "sha2"), not(feature = "shake")))]
pub use haraka::*;
