// Backend dispatcher: re-exports per-backend implementation symbols.

#[cfg(feature = "sha2")]
pub mod sha2_backend;
#[cfg(feature = "sha2")]
pub use sha2_backend::{
    gen_message_random_impl, hash_message_impl, initialize_hash_function_impl, prf_addr_impl,
    thash_impl,
};

#[cfg(feature = "shake")]
pub mod shake_backend;
#[cfg(feature = "shake")]
pub use shake_backend::{
    gen_message_random_impl, hash_message_impl, initialize_hash_function_impl, prf_addr_impl,
    thash_impl,
};

#[cfg(feature = "blake")]
pub mod blake_backend;
#[cfg(feature = "blake")]
pub use blake_backend::{
    gen_message_random_impl, hash_message_impl, initialize_hash_function_impl, prf_addr_impl,
    thash_impl,
};

#[cfg(feature = "haraka")]
pub mod haraka_backend;
#[cfg(feature = "haraka")]
pub use haraka_backend::{
    gen_message_random_impl, hash_message_impl, initialize_hash_function_impl, prf_addr_impl,
    thash_impl,
};
