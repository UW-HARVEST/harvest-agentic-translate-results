//! Rust translation of the SPHINCS+ C reference implementation shipped in
//! `c_src/`.
//!
//! The build-time configurability of the CMake project is preserved with Cargo
//! features whose names are the lowercase CMake cache variable values:
//!
//! * `HASH_BACKEND` -> `haraka`, `sha2`, `shake`, `blake`
//! * `THASH`        -> `robust`, `simple`
//! * `SECPAR`       -> `128s`, `128f`, `192s`, `192f`, `256s`, `256f`
//!
//! Every exported symbol keeps the linker name the C preprocessor produces.
//! Most of the internal headers rename their functions through
//! `#define foo SPX_NAMESPACE(foo)` with `SPX_NAMESPACE(s) == SPX_##s`, so for
//! example `thash` is exported as `SPX_thash`.  The `crypto_sign_*` API, the
//! NIST DRBG in `rng.c` and the raw hash primitives (`sha256`, `shake256`,
//! `blake512`, ...) are not renamed and keep their plain names.

#![allow(clippy::missing_safety_doc)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_range_loop)]

include!("tree.rs");
