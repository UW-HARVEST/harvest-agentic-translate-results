// Rust translation of the C library in c_src/.
//
// Original copyright notice from the C sources:
//
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

//! Public ABI of the translated library.
//!
//! The C build (`c_src/CMakeLists.txt`) compiles every file under `c_src/src`
//! into a single shared object `libhello.so`. `nm -D` on that object reports
//! exactly one public (globally defined, non-weak) symbol:
//!
//! ```text
//! 0000000000001110 T helloworld
//! ```
//!
//! `c_src/include/hello.h` declares that same single entry point:
//!
//! ```c
//! int helloworld();
//! ```
//!
//! No namespace/renaming preprocessor macros exist in the public header, so the
//! source-level name and the final linker symbol name are identical.

// Bindings to the C standard I/O routines used by the original translation
// unit. Calling into libc directly (instead of Rust's `std::io::stdout`) keeps
// the exact same `FILE *stdout` buffer, ordering and flush-at-exit semantics as
// the C library, which is what makes the emitted bytes identical when the
// library is loaded next to other C code that also prints.
pub(crate) mod cstdio;

// One Rust module per C translation unit: `c_src/src/hello.c`.
mod hello;
