#![cfg_attr(not(target_arch = "x86_64"), allow(unused))]

#[cfg(not(target_arch = "x86_64"))]
compile_error!("the generated ABI-preserving entry points require x86_64");

include!(concat!(env!("OUT_DIR"), "/exports.rs"));
