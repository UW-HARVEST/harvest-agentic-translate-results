//! Hash-backend selection. Mirrors `add_subdirectory(${HASH_BACKEND})` in
//! `c_src/lib/CMakeLists.txt` plus the `thash_<backend>_${THASH}.c` selection.
//!
//! Exactly one backend is active. When several backend features happen to be
//! enabled at once (e.g. the default `haraka` plus an explicitly requested
//! one), the precedence is sha2 > shake > blake > haraka.

// ---------------------------------------------------------------------------
// SHA2
// ---------------------------------------------------------------------------
#[cfg(feature = "sha2")]
pub mod sha2;
#[cfg(feature = "sha2")]
pub mod hash_sha2;
#[cfg(feature = "sha2")]
pub mod thash_sha2;

#[cfg(feature = "sha2")]
pub use hash_sha2::{gen_message_random, hash_message, initialize_hash_function, prf_addr};
#[cfg(feature = "sha2")]
pub use thash_sha2::thash;

// ---------------------------------------------------------------------------
// SHAKE
// ---------------------------------------------------------------------------
#[cfg(all(feature = "shake", not(feature = "sha2")))]
pub mod fips202;
#[cfg(all(feature = "shake", not(feature = "sha2")))]
pub mod hash_shake;
#[cfg(all(feature = "shake", not(feature = "sha2")))]
pub mod thash_shake;

#[cfg(all(feature = "shake", not(feature = "sha2")))]
pub use hash_shake::{gen_message_random, hash_message, initialize_hash_function, prf_addr};
#[cfg(all(feature = "shake", not(feature = "sha2")))]
pub use thash_shake::thash;

// ---------------------------------------------------------------------------
// BLAKE
// ---------------------------------------------------------------------------
#[cfg(all(feature = "blake", not(any(feature = "sha2", feature = "shake"))))]
pub mod blake256;
#[cfg(all(feature = "blake", not(any(feature = "sha2", feature = "shake"))))]
pub mod blake512;
#[cfg(all(feature = "blake", not(any(feature = "sha2", feature = "shake"))))]
pub mod hash_blake;
#[cfg(all(feature = "blake", not(any(feature = "sha2", feature = "shake"))))]
pub mod thash_blake;

#[cfg(all(feature = "blake", not(any(feature = "sha2", feature = "shake"))))]
pub use hash_blake::{gen_message_random, hash_message, initialize_hash_function, prf_addr};
#[cfg(all(feature = "blake", not(any(feature = "sha2", feature = "shake"))))]
pub use thash_blake::thash;

// ---------------------------------------------------------------------------
// HARAKA (CMake default)
// ---------------------------------------------------------------------------
#[cfg(not(any(feature = "sha2", feature = "shake", feature = "blake")))]
pub mod haraka;
#[cfg(not(any(feature = "sha2", feature = "shake", feature = "blake")))]
pub mod hash_haraka;
#[cfg(not(any(feature = "sha2", feature = "shake", feature = "blake")))]
pub mod thash_haraka;

#[cfg(not(any(feature = "sha2", feature = "shake", feature = "blake")))]
pub use hash_haraka::{gen_message_random, hash_message, initialize_hash_function, prf_addr};
#[cfg(not(any(feature = "sha2", feature = "shake", feature = "blake")))]
pub use thash_haraka::thash;
