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

//! Shared-library face of the translation (`crate-type = ["cdylib"]`).
//!
//! Exports the same symbols `mdcore.c` contributes to a link: `op_add`,
//! `op_sub`, `op_mul`, `G_OP`, `G_OP_NAME`, `helper_call`, `helper_ptr`, and
//! `use_generated`.
//!
//! Which operation and how many unrolled steps those symbols use is fixed at
//! build time by Cargo features, mirroring the CMake cache variables:
//!
//! ```text
//! cmake -DOP=mul -DREPEAT=3   ~   cargo build --no-default-features --features mul,3
//! ```

/// `src/mdmacros.h` -- compile-time OP/REPEAT selection and the REP unrolling.
pub mod mdmacros;

/// `src/mdcore.c` -- operations, generated accumulator, globals, helpers.
pub mod mdcore;

/// `<stdio.h>` output shims used by `mdcore`.
pub mod stdio;

pub use mdcore::{helper_call, helper_ptr, op_add, op_mul, op_sub, use_generated, G_OP, G_OP_NAME};
