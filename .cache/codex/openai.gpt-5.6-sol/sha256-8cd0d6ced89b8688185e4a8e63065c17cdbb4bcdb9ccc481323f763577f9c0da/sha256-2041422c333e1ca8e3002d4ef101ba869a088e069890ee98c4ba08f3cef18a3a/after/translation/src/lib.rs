mod core;

pub use core::{
    G_OP, G_OP_NAME, helper_call, helper_ptr, op_add, op_mul, op_sub, use_generated,
};

use std::ffi::{c_char, c_int};

#[unsafe(export_name = "main")]
pub unsafe extern "C" fn exported_main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    unsafe { core::run_main(argc, argv) }
}
