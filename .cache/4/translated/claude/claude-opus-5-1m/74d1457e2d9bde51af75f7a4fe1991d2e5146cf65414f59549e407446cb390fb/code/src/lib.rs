// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! A Rust translation of the C library in `c_src/`.
//!
//! The crate is built as a `cdylib` and re-exports the complete public ABI of
//! the original shared object:
//!
//! | symbol | module |
//! |---|---|
//! | `multiply_with_static` | [`ops`] |
//! | `add_with_static` | [`ops`] |
//! | `xor_operation` | [`ops`] |
//! | `shift_with_static` | [`ops`] |
//! | `get_operation` | [`ops`] |
//! | `execute_operation` | [`ops`] |
//! | `compute_checksum` | [`state`] |
//! | `init_state` | [`state`] |
//! | `apply_operation` | [`state`] |
//! | `checkshift` | [`checkshift`] |
//!
//! Behaviour - including the exact text, ordering and stdio buffering of every
//! diagnostic message, and the wrapping arithmetic on overflow - is preserved
//! verbatim. Logging goes through the platform `printf` so that output
//! interleaves with a C caller's own `printf` identically to the original.

pub mod checkshift;
pub mod cio;
pub mod ops;
pub mod state;

pub use checkshift::checkshift;
pub use ops::{
    add_with_static, execute_operation, get_operation, multiply_with_static, shift_with_static,
    xor_operation, OperationFunc,
};
pub use state::{apply_operation, compute_checksum, init_state, ComputeState};
