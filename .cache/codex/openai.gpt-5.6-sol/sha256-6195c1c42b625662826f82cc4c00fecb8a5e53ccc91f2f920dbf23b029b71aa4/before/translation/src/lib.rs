#![allow(clippy::missing_safety_doc)]

mod abi;
pub use abi::*;

include!(concat!(env!("OUT_DIR"), "/exports.rs"));
