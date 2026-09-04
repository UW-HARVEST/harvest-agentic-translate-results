#![allow(incomplete_features)]
#![allow(clippy::needless_return)]
#![feature(generic_const_exprs)]

macro_rules! assert_unique_feature {
    () => {};
    ($first:literal $(, $rest:literal)*) => {
        $(
            #[cfg(all(feature = $first, feature = $rest))]
            compile_error!(concat!(
                "features \"", $first, "\" and \"", $rest,
                "\" are mutually exclusive"
            ));
        )*
        assert_unique_feature!($($rest),*);
    };
}

assert_unique_feature!("haraka", "sha2", "shake", "blake");
assert_unique_feature!("robust", "simple");
assert_unique_feature!("128f", "128s", "192f", "192s", "256f", "256s");

#[cfg(not(any(feature = "haraka", feature = "sha2", feature = "shake", feature = "blake")))]
compile_error!("select one hash backend feature");
#[cfg(not(any(feature = "robust", feature = "simple")))]
compile_error!("select one thash feature");
#[cfg(not(any(
    feature = "128f", feature = "128s", feature = "192f",
    feature = "192s", feature = "256f", feature = "256s"
)))]
compile_error!("select one security parameter feature");

mod address;
mod api;
mod context;
mod fors;
mod ffi;
mod hash;
mod merkle;
mod offsets;
pub mod params;
mod randombytes;
mod sign;
mod thash;
pub mod transcript;
mod utils;
mod utilsx1;
mod wots;
mod wotsx1;

#[cfg(feature = "haraka")]
mod haraka;
#[cfg(feature = "sha2")]
mod sha2;
#[cfg(feature = "blake")]
mod blake;
#[cfg(feature = "shake")]
mod fips202;

pub use api::*;
pub use params::{
    CRYPTO_BYTES, CRYPTO_PUBLICKEYBYTES, CRYPTO_SECRETKEYBYTES, CRYPTO_SEEDBYTES,
    HASH, MODE, THASH,
};

pub fn initialize_deterministic_rng(entropy_input: &[u8; 48]) {
    randombytes::randombytes_init(entropy_input, None);
}

pub fn random_bytes(output: &mut [u8]) {
    randombytes::randombytes(output, output.len());
}
