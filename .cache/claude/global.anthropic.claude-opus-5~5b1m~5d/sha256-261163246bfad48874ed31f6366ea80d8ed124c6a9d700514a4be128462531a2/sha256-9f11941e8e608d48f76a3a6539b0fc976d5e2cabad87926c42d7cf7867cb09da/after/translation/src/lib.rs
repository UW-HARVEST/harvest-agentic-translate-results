// Shared-library crate root: exposes the same public C symbols as mdcore.c.

pub mod mdconfig;
pub mod mdcore;

pub use mdcore::{helper_call, helper_ptr, op_add, op_mul, op_sub, use_generated, G_OP, G_OP_NAME};
