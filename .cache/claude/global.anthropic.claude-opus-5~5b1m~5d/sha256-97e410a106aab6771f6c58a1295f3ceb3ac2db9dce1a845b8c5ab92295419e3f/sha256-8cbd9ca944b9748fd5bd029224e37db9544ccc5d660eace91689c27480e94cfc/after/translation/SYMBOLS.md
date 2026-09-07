# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared libraries.

## C shared library

`c_src/build/libharvest-work-Q7Powp.so`

```
$ nm -D --defined-only libharvest-work-Q7Powp.so
0000000000001109 T hex2bin
```

## Rust shared library

`translation/target/release/libhex2bin_lib.so`

```
$ nm -D --defined-only libhex2bin_lib.so | grep -v ' [wWvV] '
00000000000116c0 T hex2bin
```

## Parity table

| # | C symbol | type | present in Rust `.so`? | notes |
|---|----------|------|------------------------|-------|
| 1 | `hex2bin` | `T` (global text) | YES — `T hex2bin` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn hex2bin` in `src/lib.rs` |

**Missing symbols: 0.**

The C translation unit (`c_src/src/lib.c`) contains exactly one function and
`c_src/include/lib.h` declares exactly that one function, so the whole C
library surface is a single symbol. No C module was skipped by the
translation; there is nothing to add and nothing to stub.

## Undefined (imported) symbols

The C `.so` imports exactly one non-weak symbol, `strchr@GLIBC_2.2.5`:

```
$ nm -D --undefined-only libharvest-work-Q7Powp.so
  w _ITM_deregisterTMCloneTable
  w _ITM_registerTMCloneTable
  w __cxa_finalize@GLIBC_2.2.5
  w __gmon_start__
  U strchr@GLIBC_2.2.5
```

The Rust `.so` re-implements the `strchr()`-found predicate internally
(`strchr_found` in `src/lib.rs`), so it does not import `strchr` for this
purpose. Its `nm -D --undefined-only` output is entirely **libc** entries
(`malloc`, `memcpy`, `strlen`, `read`, `mmap64`, …) plus the **libgcc
unwinder** (`_Unwind_*`) and glibc TLS/atexit hooks that the Rust standard
library pulls into every `cdylib`. None of them is a project symbol.

**0 missing/undefined non-libc symbols in the Rust `.so`.** Verified by the
`--- symbol parity ---` step of `run-tests.sh`, which diffs the two symbol
lists and requires the diff to be empty.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, therefore the
only build configuration is the default one. `cargo test
--no-default-features` is equivalent to the default build. There are no
`#[cfg(feature = ...)]` gates in `src/lib.rs` and no `#ifdef` gates in
`c_src/src/lib.c`.

## Binary executable

`c_src/CMakeLists.txt` only contains `add_library(... SHARED src/lib.c)` — no
`add_executable`. `translation/Cargo.toml` has only a `[lib]` target with
`crate-type = ["cdylib"]` — no `[[bin]]`. Therefore the "compare C and Rust
binary stdout" gate is **not applicable** to this project.
