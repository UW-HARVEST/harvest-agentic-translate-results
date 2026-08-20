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

//! Rust translation of the C `driver` library (`c_src/`).
//!
//! The C build (`c_src/CMakeLists.txt`) globs the whole of `c_src/src` into one
//! shared library. That library exports exactly four public symbols:
//!
//! ```text
//! T bad
//! T driver
//! T good
//! T printLine
//! ```
//!
//! All four are reproduced here with their exact C signatures and linker names.
//! `driver` is the only symbol declared in the public header `include/driver.h`;
//! `printLine`, `bad` and `good` have external linkage in `src/driver.c` and are
//! therefore part of the exported ABI as well. The header contains no namespace
//! or function-renaming macros, so the source-level names *are* the final linker
//! symbol names.

// The C library uses lowerCamelCase identifiers. Preserve them verbatim so the
// emitted linker symbols match the C shared library byte for byte.
#![allow(non_snake_case)]

mod driver;

pub use driver::{bad, driver, good, printLine};
