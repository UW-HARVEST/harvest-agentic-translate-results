#![allow(
    non_snake_case,
    non_upper_case_globals,
    non_camel_case_types,
    clippy::identity_op,
    clippy::needless_range_loop,
    clippy::manual_memcpy
)]

mod params;
mod blake256;
mod blake512;
mod address;
mod utils;
mod context;
mod hash_blake;
mod thash_blake_robust;
mod wots;
mod wotsx1;
mod fors;
mod utilsx1;
mod merkle;
mod rng;
mod sign;
