//! The hash backends selected by the `HASH_BACKEND` CMake cache variable
//! (`lib/CMakeLists.txt` does `add_subdirectory(${HASH_BACKEND})`, so exactly
//! one backend is ever compiled).

#[cfg(backend_blake)]
pub mod blake;
#[cfg(backend_haraka)]
pub mod haraka;
#[cfg(backend_sha2)]
pub mod sha2;
#[cfg(backend_shake)]
pub mod shake;

#[cfg(backend_blake)]
pub use blake as active;
#[cfg(backend_haraka)]
pub use haraka as active;
#[cfg(backend_sha2)]
pub use sha2 as active;
#[cfg(backend_shake)]
pub use shake as active;
