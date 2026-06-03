/*
 * Rust translation of the original C driver library.
 *
 * Copyright 2025 MIT Lincoln Laboratory
 *
 * The original C source is licensed under an MIT-style permissive license;
 * see the headers in c_src/ for the full text.
 */

pub mod logger;
pub mod task_manager;
pub mod driver;

pub use driver::driver;
