// Shared-library crate root: exposes the same public C symbols as mdcore.c.

pub mod mdconfig;
pub mod mdcore;

pub use mdcore::{g_op, g_op_name, helper_call, helper_ptr, op_add, op_mul, op_sub, use_generated};
