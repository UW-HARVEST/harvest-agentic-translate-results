// Shared module tree for the `sphincsplus` cdylib and the `driver` binary.
//
// The C project builds the same objects twice (once into the shared libraries,
// once into the `driver` executable).  `include!`-ing this file from both
// `lib.rs` and `main.rs` reproduces that without giving the library an `rlib`
// crate type.

pub mod params;

pub mod address;
pub mod context;
pub mod fors;
pub mod merkle;
pub mod randombytes;
pub mod rng;
pub mod sign;
pub mod thash;
pub mod utils;
pub mod utilsx1;
pub mod wots;
pub mod wotsx1;

// ---------------------------------------------------------------------------
// `lib/CMakeLists.txt` does `add_subdirectory(${HASH_BACKEND})`: exactly one
// hash backend is compiled and linked.  The arms are ordered so that naming a
// non-default backend feature wins over the default `haraka`.
// ---------------------------------------------------------------------------

#[cfg(feature = "blake")]
pub mod blake;
#[cfg(feature = "blake")]
pub use blake as backend;

#[cfg(all(feature = "shake", not(feature = "blake")))]
pub mod shake;
#[cfg(all(feature = "shake", not(feature = "blake")))]
pub use shake as backend;

#[cfg(all(feature = "sha2", not(any(feature = "blake", feature = "shake"))))]
pub mod sha2;
#[cfg(all(feature = "sha2", not(any(feature = "blake", feature = "shake"))))]
pub use sha2 as backend;

#[cfg(not(any(feature = "blake", feature = "shake", feature = "sha2")))]
pub mod haraka;
#[cfg(not(any(feature = "blake", feature = "shake", feature = "sha2")))]
pub use haraka as backend;

/// Resolves `randombytes()` the way the linker does in the C build.
///
/// `sphincs_core` links `randombytes.c` (/dev/urandom), `sphincs_core_det`
/// links `rng.c` (the NIST AES-CTR-DRBG).  The `driver` target links
/// `sphincs_core_det`, so the DRBG is the default here as well.
pub fn randombytes_fill(x: &mut [u8]) {
    #[cfg(feature = "urandom")]
    randombytes::randombytes_urandom(x);
    #[cfg(not(feature = "urandom"))]
    rng::randombytes_drbg(x);
}
