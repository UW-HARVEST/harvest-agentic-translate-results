// Shared library entry point.
//
// Exposes the public surface of c_src/src/mdcore.c (op_add, op_sub, op_mul,
// helper_call, helper_ptr, use_generated, G_OP, G_OP_NAME) as a cdylib.
//
// Build-time configurability from c_src/CMakeLists.txt is preserved with Cargo
// features: `add`/`sub`/`mul` for the OP cache variable and `repeat_0`..
// `repeat_7` (aliases "0".."7") for the REPEAT cache variable.

#![allow(dead_code)]

pub mod mdcore;
pub mod mdmacros;

pub use mdcore::{helper_call, helper_ptr, op_add, op_mul, op_sub, use_generated, G_OP, G_OP_NAME};
