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

//! Rust translation of the C library found in `c_src/`.
//!
//! The C library is built by globbing every translation unit in `c_src/src`
//! into a single shared object.  It currently consists of `src/long.c`, whose
//! public ABI surface is:
//!
//! | symbol                        | kind      |
//! |-------------------------------|-----------|
//! | `array`                       | data (B)  |
//! | `long_exec`                   | text (T)  |
//! | `perform_expensive_operations` | text (T) |
//!
//! Every one of those symbols is re-created here with the exact same linker
//! name, C ABI and observable behaviour (including the use of the platform
//! `srand`/`rand` PRNG and `printf` so that output is byte-identical).

#![allow(non_upper_case_globals)]

pub mod clong;
