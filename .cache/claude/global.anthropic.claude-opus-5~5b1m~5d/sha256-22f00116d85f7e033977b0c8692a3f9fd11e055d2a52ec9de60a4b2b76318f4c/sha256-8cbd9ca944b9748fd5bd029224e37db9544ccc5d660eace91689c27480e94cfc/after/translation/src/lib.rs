// Rust translation of the cJSON library (cJSON.c / cJSON.h) plus the
// accompanying test driver (test.c).
//
// The translation is intentionally a literal, behaviour preserving port of the
// original C code: the same order of validation, the same (occasionally
// surprising) arithmetic and the same use of the C standard library so that the
// generated output is byte identical to the C implementation.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub mod cjson;
pub mod driver;

pub(crate) mod cshim;
