#[cfg(all(feature = "blake", feature = "robust"))]
mod blake_robust;
#[cfg(all(feature = "blake", feature = "robust"))]
pub use blake_robust::*;
#[cfg(all(feature = "blake", not(feature = "robust")))]
mod blake_simple;
#[cfg(all(feature = "blake", not(feature = "robust")))]
pub use blake_simple::*;

#[cfg(all(not(feature = "blake"), feature = "sha2", feature = "robust"))]
mod sha2_robust;
#[cfg(all(not(feature = "blake"), feature = "sha2", feature = "robust"))]
pub use sha2_robust::*;
#[cfg(all(not(feature = "blake"), feature = "sha2", not(feature = "robust")))]
mod sha2_simple;
#[cfg(all(not(feature = "blake"), feature = "sha2", not(feature = "robust")))]
pub use sha2_simple::*;

#[cfg(all(not(feature = "blake"), not(feature = "sha2"), feature = "shake", feature = "robust"))]
mod shake_robust;
#[cfg(all(not(feature = "blake"), not(feature = "sha2"), feature = "shake", feature = "robust"))]
pub use shake_robust::*;
#[cfg(all(not(feature = "blake"), not(feature = "sha2"), feature = "shake", not(feature = "robust")))]
mod shake_simple;
#[cfg(all(not(feature = "blake"), not(feature = "sha2"), feature = "shake", not(feature = "robust")))]
pub use shake_simple::*;

#[cfg(all(not(feature = "blake"), not(feature = "sha2"), not(feature = "shake"), feature = "robust"))]
mod haraka_robust;
#[cfg(all(not(feature = "blake"), not(feature = "sha2"), not(feature = "shake"), feature = "robust"))]
pub use haraka_robust::*;
#[cfg(all(not(feature = "blake"), not(feature = "sha2"), not(feature = "shake"), not(feature = "robust")))]
mod haraka_simple;
#[cfg(all(not(feature = "blake"), not(feature = "sha2"), not(feature = "shake"), not(feature = "robust")))]
pub use haraka_simple::*;
