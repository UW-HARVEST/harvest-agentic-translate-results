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

//! Shared-library crate root: the Rust translation of the `driver` project.
//!
//! Build-time configurability mirrors `c_src/CMakeLists.txt`:
//! `-DOP=<add|sub|mul>` becomes the Cargo features `add` / `sub` / `mul`, and
//! `-DREPEAT=<n>` becomes the Cargo features `0` .. `7` (aliases
//! `repeat_0` .. `repeat_7`). Defaults: `add` and `5`.

pub mod mdcore;
pub mod mdmacros;

pub use mdcore::{helper_call, helper_ptr, op_add, op_mul, op_sub, use_generated, G_OP, G_OP_NAME};
pub use mdmacros::{OP, OP_NAME, REPEAT};
