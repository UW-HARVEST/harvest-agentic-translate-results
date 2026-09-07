pub mod fips202;
pub mod hash;

#[cfg(thash_robust)]
pub mod thash_robust;
#[cfg(thash_simple)]
pub mod thash_simple;

#[cfg(thash_robust)]
pub use thash_robust::thash;
#[cfg(thash_simple)]
pub use thash_simple::thash;
