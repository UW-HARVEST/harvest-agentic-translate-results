//! Rust translation of the C library in `c_src/`.
//!
//! Layout mirrors the C sources:
//!   * `shared`      <- include/shared.h
//!   * `read_alert`  <- include/read-alert.h, src/read-alert.c
//!   * `file_queue`  <- include/file-queue.h, src/file-queue.c
//!   * `driver`      <- src/driver.c
//!
//! The exported symbols keep their C names and signatures; none of the headers
//! define namespace-renaming macros, so the linker names are unchanged.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]

pub mod driver;
pub mod file_queue;
pub mod read_alert;
pub mod shared;
