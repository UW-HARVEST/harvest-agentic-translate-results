mod params;
mod sphincs;

pub use sphincs::{
    address, api, blake_impl, context, fors, haraka, hash, merkle, randombytes, sign, thash, utils,
    utilsx1, wots, wotsx1,
};
#[cfg(feature = "sha2")]
pub use sphincs::sha2_impl as sha2;

fn main() {
    std::process::exit(sphincs::run_driver());
}
