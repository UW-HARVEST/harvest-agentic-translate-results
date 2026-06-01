//! Rust translation of the C driver library.
//!
//! Public C-compatible API exported via cdylib.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]

pub mod shared;
pub mod read_alert;
pub mod file_queue;
pub mod driver;

// Re-export the C-visible types so they appear in the crate.
pub use read_alert::alert_data;
pub use file_queue::file_queue as file_queue_t;
