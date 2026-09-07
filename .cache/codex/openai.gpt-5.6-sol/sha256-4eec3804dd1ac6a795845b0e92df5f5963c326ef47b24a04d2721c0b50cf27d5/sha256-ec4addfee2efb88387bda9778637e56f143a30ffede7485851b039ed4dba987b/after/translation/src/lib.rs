mod params;
mod sphincs;

pub use sphincs::*;
pub use sphincs::{
    address, blake_impl, context, fors, haraka, hash, merkle, randombytes, sign, thash, utils,
    utilsx1, wots, wotsx1,
};
#[cfg(feature = "sha2")]
pub use sphincs::sha2_impl as sha2;
pub use sphincs::api;
